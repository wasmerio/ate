use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};
use derivative::Derivative;
use tokio::sync::mpsc;
use wasm_bus::abi::SerializationFormat;
use wasm_bus_process::api::Spawn;
use wasmer_wasi::{
    WasiRuntimeImplementation,
    PluggableRuntimeImplementation,
    UnsupportedVirtualBus,
    UnsupportedVirtualNetworking,
    WasiError,
    WasiThreadId,
    WasiThreadError,
};
use wasmer_vnet::VirtualNetworking;
use wasmer_vbus::{VirtualBus, VirtualBusError, SpawnOptions, VirtualBusListener, BusCallEvent, VirtualBusSpawner, SpawnOptionsConfig, BusSpawnedProcess, VirtualBusProcess, VirtualBusScope, VirtualBusInvokable, BusDataFormat, VirtualBusInvocation, FileDescriptor};

use crate::api::{System, AsyncResult};
use crate::api::abi::SystemAbiExt;
use crate::bus::{ProcessExecFactory, WasmCallerContext, EvalContextTaker, ProcessExecInvokable, LaunchContext, LaunchEnvironment};
use crate::common::MAX_MPSC;
use crate::err;
use crate::fd::{Fd, WeakFd};
use crate::pipe::ReceiverMode;

use super::{EvalContext, RuntimeBusListener, RuntimeBusFeeder, EvalResult, EvalStatus, exec_process};

#[derive(Debug, Clone)]
pub struct WasiRuntime
{
    pluggable: Arc<PluggableRuntimeImplementation>,
    forced_exit: Arc<AtomicU32>,
    process_factory: ProcessExecFactory,
    ctx: WasmCallerContext,
    feeder: RuntimeBusFeeder,
    listener: RuntimeBusListener
}

impl WasiRuntime
{
    pub fn new(
        forced_exit: &Arc<AtomicU32>,
        process_factory: ProcessExecFactory,
        ctx: WasmCallerContext
    ) -> Self {
        let (tx, rx) = mpsc::channel(MAX_MPSC);
        let pluggable = PluggableRuntimeImplementation::default();
        Self {
            pluggable: Arc::new(pluggable),
            forced_exit: forced_exit.clone(),
            process_factory,
            ctx,
            feeder: RuntimeBusFeeder {
                system: Default::default(),
                listener: tx,
            },
            listener: RuntimeBusListener {
                rx: Arc::new(Mutex::new(rx)),
            }
        }
    }
}

impl WasiRuntime
{
    pub fn take_context(&self) -> Option<EvalContext> {
        self.process_factory.take_context()
    }

    pub fn prepare_take_context(&self) -> EvalContextTaker {
        EvalContextTaker::new(&self.process_factory)
    }

    pub fn to_take_context(self: Arc<Self>) -> EvalContextTaker {
        EvalContextTaker::new(&self.process_factory)
    }

    pub fn feeder(&self) -> RuntimeBusFeeder {
        self.feeder.clone()
    }
}

impl WasiRuntimeImplementation
for WasiRuntime
{
    fn bus<'a>(&'a self) -> &'a (dyn VirtualBus) {
        self
    }
    
    fn networking<'a>(&'a self) -> &'a (dyn VirtualNetworking) {
        self.pluggable.networking.deref()
    }
    
    fn thread_generate_id(&self) -> WasiThreadId {
        self.pluggable.thread_id_seed.fetch_add(1, Ordering::Relaxed).into()
    }

    fn thread_spawn(&self, task: Box<dyn FnOnce() + Send + 'static>) -> Result<(), WasiThreadError> {
        let system = System::default();
        system.task_dedicated(Box::new(move || {
            task();
            Box::pin(async move { })
        }));
        Ok(())
    }

    #[cfg(not(target_family = "wasm"))]
    fn thread_parallelism(&self) -> Result<usize, WasiThreadError> {
        if let Ok(cnt) = std::thread::available_parallelism() {
            Ok(usize::from(cnt))
        } else {
            Err(WasiThreadError::Unsupported)
        }
    }
    
    #[cfg(target_family = "wasm")]
    fn thread_parallelism(&self) -> Result<usize, WasiThreadError> {
        return Ok(8)
    }
    
    fn yield_now(&self, _id: WasiThreadId) -> Result<(), WasiError> {
        let forced_exit = self.forced_exit.load(Ordering::Acquire);
        if forced_exit != 0 {
            return Err(WasiError::Exit(forced_exit));
        }
        std::thread::yield_now();
        Ok(())
    }
}

impl VirtualBus
for WasiRuntime
{
    fn new_spawn(&self) -> SpawnOptions {
        let spawner = RuntimeProcessSpawner {
            process_factory: self.process_factory.clone(),
        };
        SpawnOptions::new(Box::new(spawner))
    }

    fn listen<'a>(&'a self) -> Result<&'a dyn VirtualBusListener, VirtualBusError> {
        Ok(&self.listener)
    }
}

struct RuntimeProcessSpawner
{
    process_factory: ProcessExecFactory,
}

impl VirtualBusSpawner
for RuntimeProcessSpawner
{
    fn spawn(&mut self, name: &str, config: &SpawnOptionsConfig) -> Result<BusSpawnedProcess, VirtualBusError>
    {
        let conv_stdio_mode = |mode| {
            use wasmer_vfs::StdioMode as S1;
            use wasm_bus_process::prelude::StdioMode as S2;
            match mode {
                S1::Inherit => S2::Inherit,
                S1::Log => S2::Log,
                S1::Null => S2::Null,
                S1::Piped => S2::Piped,
            }
        };

        let request = wasm_bus_process::api::PoolSpawnRequest {
            spawn: Spawn {
                path: name.to_string(),
                args: config.args().clone(),
                chroot: config.chroot(),
                working_dir: config.working_dir().map(|a| a.to_string()),
                stdin_mode: conv_stdio_mode(config.stdin_mode()),
                stdout_mode: conv_stdio_mode(config.stdout_mode()),
                stderr_mode: conv_stdio_mode(config.stderr_mode()),
                pre_open: config.preopen().clone(),
            }
        };

        let (runtime_tx, runtime_rx) = mpsc::channel(1);

        let env = self.process_factory.launch_env();
        let result = self
            .process_factory
            .launch_ext(request, &env, None, None, None, true,
            move |ctx: LaunchContext| {
                let runtime_tx = runtime_tx.clone();
                Box::pin(async move {
                    let stdio = ctx.eval.stdio.clone();
                    let env = ctx.eval.env.clone().into_exported();

                    let mut show_result = false;
                    let redirect = Vec::new();

                    let (process, eval_rx, runtime) = exec_process(
                        ctx.eval,
                        &ctx.path,
                        &ctx.args,
                        &env,
                        &mut show_result,
                        stdio,
                        &redirect,
                    )
                    .await?;

                    let _ = runtime_tx.send(runtime).await;

                    eval_rx
                        .await
                        .ok_or(err::ERR_ENOEXEC)
                        .map(|(ctx, ret)|
                            EvalResult {
                                ctx,
                                status: EvalStatus::Executed {
                                    code: ret,
                                    show_result: false,
                                }
                            }
                        )
                })
            });

        let process = RuntimeSpawnedProcess {
            exit_code: None,
            finish: result.finish,
            runtime_rx: Mutex::new(runtime_rx),
            runtime: Default::default(),
        };
        Ok(
            BusSpawnedProcess {
                name: name.to_string(),
                config: config.clone(),
                inst: Box::new(process),
                stdin: Some(Box::new(Fd::new(result.stdin, None, ReceiverMode::Stream, crate::fd::FdFlag::Stdin(false)))),
                stdout: Some(Box::new(Fd::new(None, result.stdout, ReceiverMode::Stream, crate::fd::FdFlag::Stdout(false)))),
                stderr: Some(Box::new(Fd::new(None, result.stderr, ReceiverMode::Stream, crate::fd::FdFlag::Stderr(false)))),
            }
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct RuntimeSpawnedProcess
{
    exit_code: Option<u32>,
    #[derivative(Debug = "ignore")]
    finish: AsyncResult<Result<EvalResult, u32>>,
    #[derivative(Debug = "ignore")]
    runtime_rx: Mutex<mpsc::Receiver<Arc<WasiRuntime>>>,
    #[derivative(Debug = "ignore")]
    runtime: RwLock<Option<Arc<WasiRuntime>>>,
}

impl VirtualBusProcess
for RuntimeSpawnedProcess
{
    fn exit_code(&self) -> Option<u32>
    {
        self.exit_code.clone()
    }
}

impl VirtualBusScope
for RuntimeSpawnedProcess
{
    fn poll_finished(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()>
    {
        if self.exit_code.is_some() {
            return Poll::Ready(())
        }
        match self.finish.rx.poll_recv(cx) {
            Poll::Ready(Some(eval)) => {
                let code = eval
                    .map(|a| {
                        match a.status {
                            EvalStatus::Executed { code, .. } => code,
                            _ => err::ERR_ENOEXEC
                        }
                    })
                    .unwrap_or_else(|err| err);
                self.exit_code.replace(code);
                Poll::Ready(())
            },
            Poll::Ready(None) => Poll::Ready(()),
            Poll::Pending => Poll::Pending
        }
    }
}

impl VirtualBusInvokable
for RuntimeSpawnedProcess
{
    fn invoke(
        &self,
        topic_hash: u128,
        format: BusDataFormat,
        buf: Vec<u8>,
        keep_alive: bool,
    ) -> Result<Box<dyn VirtualBusInvocation + Sync>, VirtualBusError>
    {
        // Fast path
        {
            let guard = self.runtime.read().unwrap();
            if let Some(runtime) = guard.deref() {
                let feeder = runtime.feeder();
                return Ok(Box::new(feeder.call_raw(topic_hash, format, buf, keep_alive)));
            }
        }

        // Enter a write lock on the runtime (and check again as it might have changed)
        let mut guard = self.runtime.write().unwrap();
        if let Some(runtime) = guard.deref() {
            let feeder = runtime.feeder();
            return Ok(Box::new(feeder.call_raw(topic_hash, format, buf, keep_alive)));
        }

        // Slow path (wait for the runtime to be returned by the sub process after it starts
        let mut runtime_rx = self.runtime_rx.lock().unwrap();
        match runtime_rx.blocking_recv() {
            Some(runtime) => {
                guard.replace(runtime.clone());
                let feeder = runtime.feeder();
                Ok(Box::new(feeder.call_raw(topic_hash, format, buf, keep_alive)))
            },
            None => {
                Err(VirtualBusError::Aborted)
            }
        }        
    }
}
