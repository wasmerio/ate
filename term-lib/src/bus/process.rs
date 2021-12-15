use async_trait::async_trait;
use derivative::*;
use std::any::type_name;
use std::collections::HashMap;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::backend::process::*;

use super::*;
use crate::api::*;
use crate::err;
use crate::eval::*;
use crate::fd::*;
use crate::pipe::*;
use crate::reactor::*;

pub struct EvalCreated {
    pub invoker: ProcessExecInvokable,
    pub session: ProcessExecSession,
}

struct ProcessExecCreate {
    request: Spawn,
    on_stdout: Option<WasmBusCallback>,
    on_stderr: Option<WasmBusCallback>,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ProcessExecFactory {
    system: System,
    #[derivative(Debug = "ignore")]
    reactor: Arc<RwLock<Reactor>>,
    #[derivative(Debug = "ignore")]
    exec_factory: EvalFactory,
    inherit_stdin: WeakFd,
    inherit_stdout: WeakFd,
    inherit_stderr: WeakFd,
    inherit_log: WeakFd,
}

pub struct LaunchContext {
    eval: EvalContext,
    path: String,
    args: Vec<String>,
    stdin_tx: Option<mpsc::Sender<Vec<u8>>>,
    stdout_rx: Option<mpsc::Receiver<Vec<u8>>>,
    stderr_rx: Option<mpsc::Receiver<Vec<u8>>>,
    on_stdout: Option<WasmBusCallback>,
    on_stderr: Option<WasmBusCallback>,
}

impl ProcessExecFactory {
    pub fn new(
        reactor: Arc<RwLock<Reactor>>,
        exec_factory: EvalFactory,
        inherit_stdin: WeakFd,
        inherit_stdout: WeakFd,
        inherit_stderr: WeakFd,
        inherit_log: WeakFd,
    ) -> ProcessExecFactory {
        let system = System::default();
        ProcessExecFactory {
            system,
            reactor,
            exec_factory,
            inherit_stdin,
            inherit_stdout,
            inherit_stderr,
            inherit_log,
        }
    }

    pub async fn launch<T, F>(
        &self,
        request: Spawn,
        mut client_callbacks: HashMap<String, WasmBusCallback>,
        funct: F,
    ) -> Result<T, CallError>
    where
        F: Fn(LaunchContext) -> Pin<Box<dyn Future<Output = Result<T, i32>>>>,
        F: Send + 'static,
        T: Send,
    {
        // Grab the callbacks and build the requiest
        let on_stdout = client_callbacks.remove(&type_name::<DataStdout>().to_string());
        let on_stderr = client_callbacks.remove(&type_name::<DataStderr>().to_string());

        let create = ProcessExecCreate {
            request,
            on_stdout,
            on_stderr,
        };

        // Push all the cloned variables into a background thread so
        // that it does not hurt anything
        let reactor = self.reactor.clone();
        let inherit_stdin = self.inherit_stdin.upgrade();
        let inherit_stdout = self.inherit_stdout.upgrade();
        let inherit_stderr = self.inherit_stderr.upgrade();
        let inherit_log = self.inherit_log.upgrade();
        let exec_factory = self.exec_factory.clone();
        let result = self.system.spawn_dedicated(move || async move {
            let path = create.request.path;
            let args = create.request.args;
            let chroot = create.request.chroot;
            let working_dir = create.request.working_dir;
            let stdin_mode = create.request.stdin_mode;
            let stdout_mode = create.request.stdout_mode;
            let stderr_mode = create.request.stderr_mode;
            let pre_open = create.request.pre_open;
            let on_stdout = create.on_stdout;
            let on_stderr = create.on_stderr;

            // Get the current job (if there is none then fail)
            let job = {
                let reactor = reactor.read().await;
                reactor.get_current_job().ok_or(err::ERR_ECHILD)?
            };

            // Build the comand string
            let mut cmd = path.clone();
            for arg in args.iter() {
                cmd.push_str(" ");
                if arg.contains(" ")
                    && cmd.starts_with("\"") == false
                    && cmd.starts_with("'") == false
                {
                    cmd.push_str("\"");
                    cmd.push_str(arg);
                    cmd.push_str("\"");
                } else {
                    cmd.push_str(arg);
                }
            }

            // Create all the stdio
            let (stdin, stdin_tx) = pipe_in(ReceiverMode::Stream, false);
            let (stdout, stdout_rx) = pipe_out(false);
            let (stderr, stderr_rx) = pipe_out(false);

            // Perform hooks back to the main stdio
            let (stdin, stdin_tx) = match stdin_mode {
                StdioMode::Null => (stdin, None),
                StdioMode::Inherit if inherit_stdin.is_some() => (inherit_stdin.unwrap(), None),
                StdioMode::Inherit => (stdin, None),
                StdioMode::Piped => (stdin, Some(stdin_tx)),
                StdioMode::Log => (stdin, None),
            };
            let (stdout, stdout_rx) = match stdout_mode {
                StdioMode::Null => (stdout, None),
                StdioMode::Inherit if inherit_stdout.is_some() => (inherit_stdout.unwrap(), None),
                StdioMode::Inherit => (stdout, None),
                StdioMode::Piped => (stdout, Some(stdout_rx)),
                StdioMode::Log if inherit_log.is_some() => (inherit_log.clone().unwrap(), None),
                StdioMode::Log => (stdout, None),
            };
            let (stderr, stderr_rx) = match stderr_mode {
                StdioMode::Null => (stderr, None),
                StdioMode::Inherit if inherit_stderr.is_some() => (inherit_stderr.unwrap(), None),
                StdioMode::Inherit => (stderr, None),
                StdioMode::Piped => (stderr, Some(stderr_rx)),
                StdioMode::Log if inherit_log.is_some() => (inherit_log.clone().unwrap(), None),
                StdioMode::Log => (stderr, None),
            };

            // Create the eval context
            let spawn = SpawnContext::new(
                cmd,
                job.env.deref().clone(),
                job.clone(),
                stdin,
                stdout,
                stderr,
                chroot,
                working_dir
                    .as_ref()
                    .map(|a| a.clone())
                    .unwrap_or("/".to_string()),
                pre_open,
                job.root.clone(),
            );
            let eval = exec_factory.create_context(spawn);

            // Build a context
            let ctx = LaunchContext {
                eval,
                path,
                args,
                stdin_tx,
                stdout_rx,
                stderr_rx,
                on_stdout,
                on_stderr,
            };

            // Start the process
            Ok(funct(ctx).await?)
        });

        let ret = match result.join().await.ok_or_else(|| CallError::Aborted)? {
            Ok(created) => created,
            Err(err) => {
                let err: i32 = err;
                warn!("failed to created process - internal error - code={}", err);
                return Err(CallError::Unknown);
            }
        };

        Ok(ret)
    }

    pub async fn eval(
        &self,
        request: Spawn,
        client_callbacks: HashMap<String, WasmBusCallback>,
    ) -> Result<EvalCreated, CallError> {
        self.launch(request, client_callbacks, |ctx: LaunchContext| {
            Box::pin(async move {
                let eval_rx = crate::eval::eval(ctx.eval);

                Ok(EvalCreated {
                    invoker: ProcessExecInvokable {
                        stdout: ctx.stdout_rx,
                        stderr: ctx.stderr_rx,
                        eval_rx: Some(eval_rx),
                        on_stdout: ctx.on_stdout,
                        on_stderr: ctx.on_stderr,
                    },
                    session: ProcessExecSession {
                        stdin: ctx.stdin_tx,
                    },
                })
            })
        })
        .await
    }

    pub async fn create(
        &self,
        request: Spawn,
        client_callbacks: HashMap<String, WasmBusCallback>,
    ) -> Result<(Process, AsyncResult<i32>, Arc<WasmBusThreadPool>), CallError> {
        self.launch(request, client_callbacks, |mut ctx: LaunchContext| {
            Box::pin(async move {
                let stdio = ctx.eval.stdio.clone();
                let env = ctx.eval.env.clone().into_exported();

                let mut show_result = false;
                let redirect = Vec::new();

                let ret = exec_process(
                    &mut ctx.eval,
                    &ctx.path,
                    &ctx.args,
                    &env,
                    &mut show_result,
                    stdio,
                    &redirect,
                )
                .await?;

                Ok(ret)
            })
        })
        .await
    }
}

pub struct ProcessExecInvokable {
    stdout: Option<mpsc::Receiver<Vec<u8>>>,
    stderr: Option<mpsc::Receiver<Vec<u8>>>,
    eval_rx: Option<mpsc::Receiver<EvalPlan>>,
    on_stdout: Option<WasmBusCallback>,
    on_stderr: Option<WasmBusCallback>,
}

#[async_trait]
impl Invokable for ProcessExecInvokable {
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        // Get the eval_rx (this will mean it is destroyed when this
        // function returns)
        let mut eval_rx = match self.eval_rx.take() {
            Some(a) => a,
            None => {
                return encode_eval_response(Some(EvalPlan::InternalError));
            }
        };

        // Now process all the STDIO concurrently
        loop {
            if let Some(stdout_rx) = self.stdout.as_mut() {
                if let Some(stderr_rx) = self.stderr.as_mut() {
                    tokio::select! {
                        data = stdout_rx.recv() => {
                            if let (Some(data), Some(on_data)) = (data, self.on_stdout.as_mut()) {
                                on_data.feed(DataStdout(data));
                            } else {
                                self.stdout.take();
                            }
                        }
                        data = stderr_rx.recv() => {
                            if let (Some(data), Some(on_data)) = (data, self.on_stderr.as_mut()) {
                                on_data.feed(DataStderr(data));
                            } else {
                                self.stderr.take();
                            }
                        }
                        res = eval_rx.recv() => {
                            return encode_eval_response(res);
                        }
                    }
                } else {
                    tokio::select! {
                        data = stdout_rx.recv() => {
                            if let (Some(data), Some(on_data)) = (data, self.on_stdout.as_mut()) {
                                on_data.feed(DataStdout(data));
                            } else {
                                self.stdout.take();
                            }
                        }
                        res = eval_rx.recv() => {
                            return encode_eval_response(res);
                        }
                    }
                }
            } else {
                if let Some(stderr_rx) = self.stderr.as_mut() {
                    tokio::select! {
                        data = stderr_rx.recv() => {
                            if let (Some(data), Some(on_data)) = (data, self.on_stderr.as_mut()) {
                                on_data.feed(DataStderr(data));
                            } else {
                                self.stderr.take();
                            }
                        }
                        res = eval_rx.recv() => {
                            return encode_eval_response(res);
                        }
                    }
                } else {
                    tokio::select! {
                        res = eval_rx.recv() => {
                            return encode_eval_response(res);
                        }
                    }
                }
            }
        }
    }
}

fn encode_eval_response(res: Option<EvalPlan>) -> Result<Vec<u8>, CallError> {
    Ok(encode_response(&match res {
        Some(EvalPlan::Executed { code, .. }) => ProcessExited { exit_code: code },
        Some(EvalPlan::InternalError) => ProcessExited {
            exit_code: err::ERR_ENOEXEC,
        },
        Some(EvalPlan::Invalid) => ProcessExited {
            exit_code: err::ERR_EINVAL,
        },
        Some(EvalPlan::MoreInput) => ProcessExited {
            exit_code: err::ERR_EINVAL,
        },
        None => ProcessExited {
            exit_code: err::ERR_EPIPE,
        },
    })?)
}

#[derive(Clone)]
pub struct ProcessExecSession {
    stdin: Option<mpsc::Sender<Vec<u8>>>,
}

impl Session for ProcessExecSession {
    fn call(&mut self, topic: &str, request: Vec<u8>) -> Box<dyn Invokable + 'static> {
        if topic == type_name::<OutOfBand>() {
            let request: OutOfBand = match decode_request(request.as_ref()) {
                Ok(a) => a,
                Err(err) => {
                    return ErrornousInvokable::new(err);
                }
            };
            match request {
                OutOfBand::DataStdin(data) => {
                    if let Some(stdin) = self.stdin.as_ref() {
                        let tx_send = stdin.clone();
                        let _ = tx_send.blocking_send(data);
                    }
                }
                OutOfBand::CloseStdin => {
                    self.stdin.take();
                }
                _ => {}
            }
            ResultInvokable::new(())
        } else {
            ErrornousInvokable::new(CallError::InvalidTopic)
        }
    }
}
