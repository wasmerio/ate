use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};
use derivative::Derivative;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use wasmer::{Module, Store};
use wasmer::vm::VMMemory;
use wasmer_bus::abi::SerializationFormat;
use wasmer_bus_process::api::Spawn;
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
use wasmer_vbus::{VirtualBus, VirtualBusError, SpawnOptions, VirtualBusListener, BusCallEvent, VirtualBusSpawner, SpawnOptionsConfig, BusSpawnedProcess, VirtualBusProcess, VirtualBusScope, VirtualBusInvokable, BusDataFormat, VirtualBusInvocation, FileDescriptor, BusInvocationEvent, VirtualBusInvoked};

use crate::api::{System, AsyncResult};
use crate::api::abi::{SystemAbiExt, SpawnType};
use crate::bus::{ProcessExecFactory, WasmCallerContext, EvalContextTaker, ProcessExecInvokable, LaunchContext, LaunchEnvironment, StandardBus, LaunchResult, WasmCheckpoint};
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

    fn thread_spawn(&self, task: Box<dyn FnOnce(Store, Module, VMMemory) + Send + 'static>, store: Store, module: Module, memory: VMMemory) -> Result<(), WasiThreadError> {
        let system = System::default();
        system.task_wasm(Box::new(move |store, module, memory| {
                task(store, module, memory.expect("failed to use existing memory"));
                Box::pin(async move { })
            }),
            store,
            module,
            SpawnType::NewThread(memory))
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
        return Ok(0)
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

pub(crate) struct RuntimeProcessSpawner
{
    pub(crate) process_factory: ProcessExecFactory,
}

struct RuntimeProcessSpawned
{
    pub result: LaunchResult<EvalResult>,
    pub runtime: mpsc::Receiver<Arc<WasiRuntime>>,
}

impl RuntimeProcessSpawner
{
    pub fn spawn(&mut self, name: &str, config: &SpawnOptionsConfig) -> Result<LaunchResult<EvalResult>, VirtualBusError>
    {
        let spawned = self.spawn_internal(name, config)?;
        Ok(spawned.result)
    }
    
    fn spawn_internal(&mut self, name: &str, config: &SpawnOptionsConfig) -> Result<RuntimeProcessSpawned, VirtualBusError>
    {
        let conv_stdio_mode = |mode| {
            use wasmer_vfs::StdioMode as S1;
            use wasmer_bus_process::prelude::StdioMode as S2;
            match mode {
                S1::Inherit => S2::Inherit,
                S1::Log => S2::Log,
                S1::Null => S2::Null,
                S1::Piped => S2::Piped,
            }
        };

        let request = wasmer_bus_process::api::PoolSpawnRequest {
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

                    let (process, eval_rx, runtime, checkpoint2) = exec_process(
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

        Ok(
            RuntimeProcessSpawned {
                result,
                runtime: runtime_rx,
            }
        )
    }
} 

impl VirtualBusSpawner
for RuntimeProcessSpawner
{
    fn spawn(&mut self, name: &str, config: &SpawnOptionsConfig) -> Result<BusSpawnedProcess, VirtualBusError>
    {
        if name == "os" {
            let env = self.process_factory.launch_env();
            return Ok(BusSpawnedProcess {
                name: "os".to_string(),
                config: config.clone(),
                inst: Box::new(
                    StandardBus::new(self.process_factory.clone())
                ),
                stdin: Some(Box::new(self.process_factory.stdin(&env))),
                stdout: Some(Box::new(self.process_factory.stdout(&env).fd())),
                stderr: Some(Box::new(self.process_factory.stderr(&env))),
            });
        }

        let spawned = RuntimeProcessSpawner::spawn_internal(self, name, config)?;

        let process = RuntimeSpawnedProcess {
            exit_code: None,
            finish: spawned.result.finish,
            checkpoint2: spawned.result.checkpoint2,
            runtime: Arc::new(
                DelayedRuntime {
                    rx: Mutex::new(spawned.runtime),
                    val: RwLock::new(None)
                }
            )
        };

        Ok(
            BusSpawnedProcess {
                name: name.to_string(),
                config: config.clone(),
                inst: Box::new(process),
                stdin: Some(Box::new(Fd::new(spawned.result.stdin, None, ReceiverMode::Stream, crate::fd::FdFlag::Stdin(false)))),
                stdout: Some(Box::new(Fd::new(None, spawned.result.stdout, ReceiverMode::Stream, crate::fd::FdFlag::Stdout(false)))),
                stderr: Some(Box::new(Fd::new(None, spawned.result.stderr, ReceiverMode::Stream, crate::fd::FdFlag::Stderr(false)))),
            }
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct DelayedRuntime
{
    #[derivative(Debug = "ignore")]
    rx: Mutex<mpsc::Receiver<Arc<WasiRuntime>>>,
    #[derivative(Debug = "ignore")]
    val: RwLock<Option<Result<Arc<WasiRuntime>, VirtualBusError>>>,
}

impl DelayedRuntime
{
    fn poll_runtime(&self, cx: &mut Context<'_>) -> Poll<Result<Arc<WasiRuntime>, VirtualBusError>>
    {
        // Fast path
        {
            let guard = self.val.read().unwrap();
            if let Some(runtime) = guard.deref() {
                return Poll::Ready(runtime.clone());
            }
        }

        // Enter a write lock on the runtime (and check again as it might have changed)
        let mut guard = self.val.write().unwrap();
        if let Some(runtime) = guard.deref() {
            return Poll::Ready(runtime.clone());
        }

        // Slow path (wait for the runtime to be returned by the sub process after it starts
        let mut runtime_rx = self.rx.lock().unwrap();
        match runtime_rx.poll_recv(cx) {
            Poll::Ready(runtime) => {
                match runtime {
                    Some(runtime) => {
                        guard.replace(Ok(runtime.clone()));
                        Poll::Ready(Ok(runtime))
                    },
                    None => {
                        guard.replace(Err(VirtualBusError::Aborted));
                        Poll::Ready(Err(VirtualBusError::Aborted))
                    }
                }
            },
            Poll::Pending => Poll::Pending
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct RuntimeSpawnedProcess
{
    exit_code: Option<u32>,
    #[derivative(Debug = "ignore")]
    finish: AsyncResult<Result<EvalResult, u32>>,
    checkpoint2: Arc<WasmCheckpoint>,
    runtime: Arc<DelayedRuntime>,
}

impl VirtualBusProcess
for RuntimeSpawnedProcess
{
    fn exit_code(&self) -> Option<u32>
    {
        self.exit_code.clone()
    }

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let checkpoint2 = Pin::new(self.checkpoint2.deref());
        checkpoint2.poll(cx)
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
    ) -> Box<dyn VirtualBusInvoked>
    {
        Box::new(
            DelayedInvocation {
                topic_hash,
                format,
                buf: Some(buf),
                runtime: self.runtime.clone()
            }
        )
    }
}

#[derive(Debug)]
struct DelayedInvocation
{
    topic_hash: u128,
    format: BusDataFormat,
    buf: Option<Vec<u8>>,
    runtime: Arc<DelayedRuntime>,
}

impl VirtualBusInvoked
for DelayedInvocation
{
    fn poll_invoked(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Box<dyn VirtualBusInvocation + Sync>, VirtualBusError>> {
        let runtime = match self.runtime.poll_runtime(cx) {
            Poll::Ready(Ok(runtime)) => runtime,
            Poll::Ready(Err(err)) => { return Poll::Ready(Err(err)); },
            Poll::Pending => { return Poll::Pending; }
        };

        let buf = match self.buf.take() {
            Some(a) => a,
            None => {
                return Poll::Ready(Err(VirtualBusError::AlreadyConsumed));
            }
        };

        let feeder = runtime.feeder();
        Poll::Ready(
            Ok(
                Box::new(feeder.call_raw(self.topic_hash, self.format, buf))
            )
        )
    }
}
