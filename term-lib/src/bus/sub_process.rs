#![allow(dead_code)]
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::task::Context;
use std::task::Poll;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus::abi::SerializationFormat;
use wasm_bus_process::api;
use wasm_bus_process::prelude::*;
use wasmer_vbus::BusDataFormat;
use wasmer_vbus::BusInvocationEvent;
use wasmer_vbus::InstantInvocation;
use wasmer_vbus::SpawnOptionsConfig;
use wasmer_vbus::VirtualBusError;
use wasmer_vbus::VirtualBusInvocation;
use wasmer_vbus::VirtualBusInvokable;
use wasmer_vbus::VirtualBusInvoked;

use crate::api::AsyncResult;
use crate::api::System;
use crate::eval::EvalContext;
use crate::eval::EvalResult;
use crate::eval::Process;
use crate::eval::RuntimeCallOutsideTask;
use crate::eval::RuntimeProcessSpawner;
use crate::eval::WasiRuntime;
use crate::fd::FdMsg;

use super::*;

#[derive(Clone)]
pub struct SubProcessMultiplexer {
    processes: Arc<Mutex<HashMap<String, Weak<SubProcess>>>>,
}

impl SubProcessMultiplexer {
    pub fn new() -> SubProcessMultiplexer {
        SubProcessMultiplexer {
            processes: Arc::new(Mutex::new(HashMap::default())),
        }
    }
}

pub struct SubProcessFactoryInner {
    process_factory: ProcessExecFactory,
    multiplexer: SubProcessMultiplexer,
}

#[derive(Clone)]
pub struct SubProcessFactory {
    inner: Arc<SubProcessFactoryInner>,
    ctx: Arc<Mutex<Option<EvalContext>>>,
}

impl SubProcessFactory {
    pub fn new(process_factory: ProcessExecFactory, multiplexer: SubProcessMultiplexer) -> SubProcessFactory {
        SubProcessFactory {
            ctx: process_factory.ctx.clone(),
            inner: Arc::new(SubProcessFactoryInner {
                process_factory,
                multiplexer,
            }),
        }
    }
    pub async fn get_or_create(
        &self,
        wapm: &str,
        env: &LaunchEnvironment,
        stdout_mode: StdioMode,
        stderr_mode: StdioMode,
    ) -> Result<Arc<SubProcess>, BusError> {
        let wapm = wapm.to_string();
        let key = format!("{}-{}-{}", wapm, stdout_mode, stderr_mode);

        // Check for any existing process of this name thats already running
        {
            let processes = self.inner.multiplexer.processes.lock().unwrap();
            if let Some(process) = processes
                .get(&key)
                .iter()
                .filter_map(|a| a.upgrade())
                .next()
            {
                return Ok(process);
            }
        }

        // None was found so go ahead and start a new process
        let empty_client_callbacks = HashMap::default();
        let spawn = api::PoolSpawnRequest {
            spawn: api::Spawn {
                path: wapm.clone(),
                args: vec![wapm.to_string(), "bus".to_string()],
                chroot: false,
                working_dir: None,
                stdin_mode: StdioMode::Null,
                stdout_mode: stdout_mode,
                stderr_mode: stderr_mode,
                pre_open: Vec::new(),
            },
        };
        let (process, process_result, runtime) = self
            .inner
            .process_factory
            .create(spawn, &env, empty_client_callbacks)
            .await?;

        // Add it to the list of sub processes and return it
        let ctx = self.ctx.clone();
        let process = Arc::new(SubProcess::new(
            wapm.as_str(),
            process,
            process_result,
            runtime,
            ctx,
        ));
        {
            let mut processes = self.inner.multiplexer.processes.lock().unwrap();
            processes.insert(key, Arc::downgrade(&process));
        }
        Ok(process)
    }
}

pub struct SubProcessInner {
    pub wapm: String,
}

pub struct SubProcess {
    pub system: System,
    pub process: Process,
    pub process_result: Arc<Mutex<AsyncResult<(EvalContext, u32)>>>,
    pub inner: Arc<SubProcessInner>,
    pub runtime: Arc<WasiRuntime>,
    pub ctx: Arc<Mutex<Option<EvalContext>>>,
}

impl SubProcess {
    pub fn new(
        wapm: &str,
        process: Process,
        process_result: AsyncResult<(EvalContext, u32)>,
        runtime: Arc<WasiRuntime>,
        ctx: Arc<Mutex<Option<EvalContext>>>,
    ) -> SubProcess {
        SubProcess {
            system: System::default(),
            process,
            process_result: Arc::new(Mutex::new(process_result)),
            inner: Arc::new(SubProcessInner {
                wapm: wapm.to_string(),
            }),
            runtime,
            ctx,
        }
    }

    pub fn create(
        self: &Arc<Self>,
        topic_hash: u128,
        format: BusDataFormat,
        request: Vec<u8>,
        ctx: WasmCallerContext,
        _client_callbacks: HashMap<String, Arc<dyn BusStatefulFeeder + Send + Sync + 'static>>,
    ) -> Result<(Box<dyn Processable>, Option<Box<dyn Session>>), BusError> {
        let feeder = self.runtime.feeder();
        let handle = feeder.call_raw(topic_hash, format, request);
        let sub_process = self.clone();

        let session = SubProcessSession::new(self.runtime.clone(), handle.clone_task(), sub_process, ctx);
        Ok((Box::new(handle), Some(Box::new(session))))
    }
}

pub struct SubProcessSession {
    pub runtime: Arc<WasiRuntime>,
    pub task: RuntimeCallOutsideTask,
    pub sub_process: Arc<SubProcess>,
    pub ctx: WasmCallerContext,
}

impl SubProcessSession {
    pub fn new(
        runtime: Arc<WasiRuntime>,
        task: RuntimeCallOutsideTask,
        sub_process: Arc<SubProcess>,
        ctx: WasmCallerContext,
    ) -> SubProcessSession {
        SubProcessSession {
            runtime,
            task,
            sub_process,
            ctx,
        }
    }
}

impl Session for SubProcessSession {
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        let invoker =
            self.task
                .call_raw(topic_hash, format, request);
        Ok((Box::new(invoker), None))
    }
}

pub fn process_spawn(
    factory: ProcessExecFactory,
    request: api::PoolSpawnRequest,
) -> Box<dyn VirtualBusInvoked> {
    let mut spawner = RuntimeProcessSpawner {
        process_factory: factory
    };

    let conv_stdio_mode = |mode: wasm_bus_process::prelude::StdioMode| -> wasmer_vfs::StdioMode {
        use wasm_bus_process::prelude::StdioMode::*;
        match mode {
            Piped => wasmer_vfs::StdioMode::Piped,
            Inherit => wasmer_vfs::StdioMode::Inherit,
            Null => wasmer_vfs::StdioMode::Null,
            Log => wasmer_vfs::StdioMode::Log,
        }
    };

    let config = SpawnOptionsConfig {
        reuse: false,
        chroot: request.spawn.chroot,
        args: request.spawn.args,
        preopen: request.spawn.pre_open,
        stdin_mode: conv_stdio_mode(request.spawn.stdin_mode),
        stdout_mode: conv_stdio_mode(request.spawn.stdout_mode),
        stderr_mode: conv_stdio_mode(request.spawn.stderr_mode),
        working_dir: request.spawn.working_dir,
        remote_instance: None,
        access_token: None,
    };
    
    let result = match spawner.spawn(request.spawn.path.as_str(), &config) {
        Ok(a) => a,
        Err(err) => {
            return Box::new(InstantInvocation::fault(err));
        }
    };
    Box::new(InstantInvocation::call(
        Box::new(SubProcessHandler {
            result: Mutex::new(result)
        })
    ))
}

#[derive(Debug)]
pub struct SubProcessHandler {
    result: Mutex<LaunchResult<EvalResult>>
}

impl VirtualBusInvocation
for SubProcessHandler
{
    fn poll_event(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<BusInvocationEvent> {
        let mut result = self.result.lock().unwrap();
        if let Some(stdout) = &mut result.stdout {
            let mut stdout = Pin::new(stdout);
            while let Poll::Ready(msg) = stdout.poll_recv(cx) {
                if let Some(FdMsg::Data { data, flag }) = msg {
                    if flag.is_stdin() {
                        let data = api::PoolSpawnStdoutCallback(data);
                        return Poll::Ready(BusInvocationEvent::Callback {
                            topic_hash: type_name_hash::<api::PoolSpawnStdoutCallback>(),
                            format: BusDataFormat::Bincode,
                            data: match SerializationFormat::Bincode.serialize(data) {
                                Ok(d) => d,
                                Err(err) => {
                                    return Poll::Ready(BusInvocationEvent::Fault { fault: conv_error_back(err) });
                                }
                            }
                        });
                    }
                } else {
                    break;
                }
            }
        }
        if let Some(stderr) = &mut result.stderr {
            let mut stderr = Pin::new(stderr);
            while let Poll::Ready(msg) = stderr.poll_recv(cx) {
                if let Some(FdMsg::Data { data, flag }) = msg {
                    if flag.is_stderr() {
                        let data = api::PoolSpawnStderrCallback(data);
                        return Poll::Ready(BusInvocationEvent::Callback {
                            topic_hash: type_name_hash::<api::PoolSpawnStderrCallback>(),
                            format: BusDataFormat::Bincode,
                            data: match SerializationFormat::Bincode.serialize(data) {
                                Ok(d) => d,
                                Err(err) => {
                                    return Poll::Ready(BusInvocationEvent::Fault { fault: conv_error_back(err) });
                                }
                            }
                        });
                    }
                } else {
                    break;
                }
            }
        }
        let finish = Pin::new(&mut result.finish);
        if let Poll::Ready(finish) = finish.poll(cx) {
            if let Some(finish) = finish {
                let code = finish.map(|a| a.raw()).unwrap_or_else(|a| a);
                let data = api::PoolSpawnExitCallback(code as i32);
                return Poll::Ready(BusInvocationEvent::Callback {
                    topic_hash: type_name_hash::<api::PoolSpawnExitCallback>(),
                    format: BusDataFormat::Bincode,
                    data: match SerializationFormat::Bincode.serialize(data) {
                        Ok(d) => d,
                        Err(err) => {
                            return Poll::Ready(BusInvocationEvent::Fault { fault: conv_error_back(err) });
                        }
                    }
                });
            } else {
                return Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::Aborted });
            }
        }
        Poll::Pending
    }
}

impl VirtualBusInvokable
for SubProcessHandler
{
    fn invoke(
        &self,
        topic_hash: u128,
        format: BusDataFormat,
        buf: Vec<u8>,
    ) -> Box<dyn VirtualBusInvoked>
    {
        let mut result = self.result.lock().unwrap();
        if topic_hash == type_name_hash::<api::ProcessStdinRequest>() {
            let data = match decode_request::<api::ProcessStdinRequest>(
                format,
                buf,
            ) {
                Ok(a) => a.data,
                Err(err) => {
                    return Box::new(InstantInvocation::fault(conv_error_back(err)));
                }
            };
            let data_len = data.len();
            
            if let Some(stdin) = &result.stdin {
                match stdin.blocking_send(FdMsg::Data { data, flag: crate::fd::FdFlag::Stdin(false) }) {
                    Ok(_) => {
                        Box::new(encode_instant_response(BusDataFormat::Bincode, &data_len))
                    }
                    Err(err) => {
                        debug!("failed to send data to stdin of process - {}", err);
                        Box::new(InstantInvocation::fault(VirtualBusError::InternalError))
                    }
                }
                
            } else {
                Box::new(InstantInvocation::fault(VirtualBusError::BadHandle))
            }
        } else if topic_hash == type_name_hash::<api::ProcessCloseStdinRequest>() {
            result.stdin.take();
            Box::new(encode_instant_response(BusDataFormat::Bincode, &()))
        } else if topic_hash == type_name_hash::<api::ProcessFlushRequest>() {
            Box::new(encode_instant_response(BusDataFormat::Bincode, &()))
        } else if topic_hash == type_name_hash::<api::ProcessIdRequest>() {
            let id = 0u32;
            Box::new(encode_instant_response(BusDataFormat::Bincode, &id))
        } else {
            debug!("websocket invalid topic (hash={})", topic_hash);
            Box::new(InstantInvocation::fault(VirtualBusError::InvalidTopic))
        }
    }
}
