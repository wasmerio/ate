use crate::common::MAX_MPSC;
use async_trait::async_trait;
use std::any::type_name;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::backend::process::*;

use super::*;
use crate::err;
use crate::eval::*;
use crate::fd::*;
use crate::pipe::*;
use crate::reactor::*;

struct ProcessCreated {
    invoker: ProcessExecInvokable,
    session: ProcessExecSession,
}

struct ProcessExecCreate {
    request: Spawn,
    result: mpsc::Sender<Result<ProcessCreated, i32>>,
    on_stdout: Option<WasmBusFeeder>,
    on_stderr: Option<WasmBusFeeder>,
}

#[derive(Debug, Clone)]
pub struct ProcessExecFactory {
    maker: mpsc::Sender<ProcessExecCreate>,
}

impl ProcessExecFactory {
    pub fn new(
        reactor: Arc<RwLock<Reactor>>,
        exec_factory: ExecFactory,
        inherit_stdin: WeakFd,
        inherit_stdout: WeakFd,
        inherit_stderr: WeakFd,
    ) -> ProcessExecFactory {
        let (tx_factory, mut rx_factory) = mpsc::channel::<ProcessExecCreate>(MAX_MPSC);
        wasm_bindgen_futures::spawn_local(async move {
            while let Some(create) = rx_factory.recv().await {
                let reactor = reactor.clone();
                let inherit_stdin = inherit_stdin.upgrade();
                let inherit_stdout = inherit_stdout.upgrade();
                let inherit_stderr = inherit_stderr.upgrade();
                let exec_factory = exec_factory.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let path = create.request.path;
                    let args = create.request.args;
                    let current_dir = create.request.current_dir;
                    let stdin_mode = create.request.stdin_mode;
                    let stdout_mode = create.request.stdout_mode;
                    let stderr_mode = create.request.stderr_mode;
                    let pre_open = create.request.pre_open;
                    let reactor = reactor.clone();
                    let on_stdout = create.on_stdout;
                    let on_stderr = create.on_stderr;

                    let resp = move || async move {
                        // Build the comand string
                        let mut cmd = path.clone();
                        for arg in args {
                            cmd.push_str(" ");
                            if arg.contains(" ")
                                && cmd.starts_with("\"") == false
                                && cmd.starts_with("'") == false
                            {
                                cmd.push_str("\"");
                                cmd.push_str(&arg);
                                cmd.push_str("\"");
                            } else {
                                cmd.push_str(&arg);
                            }
                        }

                        // Get the current job (if there is none then fail)
                        let job = {
                            let reactor = reactor.read().await;
                            reactor.get_current_job().ok_or(err::ERR_ECHILD)?
                        };

                        // Create all the stdio
                        let (stdin, stdin_tx) = pipe_in(ReceiverMode::Stream);
                        let (stdout, stdout_rx) = pipe_out();
                        let (stderr, stderr_rx) = pipe_out();

                        // Perform hooks back to the main stdio
                        let (stdin, stdin_tx) = match stdin_mode {
                            StdioMode::Null => (stdin, None),
                            StdioMode::Inherit if inherit_stdin.is_some() => {
                                (inherit_stdin.unwrap(), None)
                            }
                            StdioMode::Inherit => (stdin, None),
                            StdioMode::Piped => (stdin, Some(stdin_tx)),
                        };
                        let (stdout, stdout_rx) = match stdout_mode {
                            StdioMode::Null => (stdout, None),
                            StdioMode::Inherit if inherit_stdout.is_some() => {
                                (inherit_stdout.unwrap(), None)
                            }
                            StdioMode::Inherit => (stdout, None),
                            StdioMode::Piped => (stdout, Some(stdout_rx)),
                        };
                        let (stderr, stderr_rx) = match stderr_mode {
                            StdioMode::Null => (stderr, None),
                            StdioMode::Inherit if inherit_stderr.is_some() => {
                                (inherit_stderr.unwrap(), None)
                            }
                            StdioMode::Inherit => (stderr, None),
                            StdioMode::Piped => (stderr, Some(stderr_rx)),
                        };

                        // Build a context
                        let ctx = SpawnContext::new(
                            cmd,
                            job.env.deref().clone(),
                            job.clone(),
                            stdin,
                            stdout,
                            stderr,
                            current_dir.unwrap_or(job.working_dir.clone()),
                            pre_open,
                            job.root.clone(),
                        );

                        // Start the process
                        let eval_rx = exec_factory.spawn(ctx).await;

                        // Build the invokable process and return it to the caller
                        let ret = ProcessCreated {
                            invoker: ProcessExecInvokable {
                                stdout: stdout_rx,
                                stderr: stderr_rx,
                                eval_rx: Some(eval_rx),
                                on_stdout,
                                on_stderr,
                            },
                            session: ProcessExecSession { stdin: stdin_tx },
                        };

                        // We are done (this will close all the pipes)
                        Ok(ret)
                    };
                    let _ = create.result.send(resp().await).await;
                });
            }
        });
        ProcessExecFactory { maker: tx_factory }
    }

    pub fn create(
        &self,
        request: Spawn,
        mut client_callbacks: HashMap<String, WasmBusFeeder>,
    ) -> Result<(Box<dyn Invokable>, Option<Box<dyn Session>>), CallError> {
        let on_stdout = client_callbacks.remove(&type_name::<DataStdout>().to_string());
        let on_stderr = client_callbacks.remove(&type_name::<DataStderr>().to_string());

        let (tx_result, mut rx_result) = mpsc::channel(1);
        let request = ProcessExecCreate {
            request,
            result: tx_result,
            on_stdout,
            on_stderr,
        };

        let _ = self.maker.blocking_send(request);

        let ret = match rx_result
            .blocking_recv()
            .ok_or_else(|| CallError::Aborted)?
        {
            Ok(created) => created,
            Err(err) => {
                warn!("failed to created process - internal error - code={}", err);
                return Ok((ErrornousInvokable::new(CallError::Unknown), None));
            }
        };

        Ok((Box::new(ret.invoker), Some(Box::new(ret.session))))
    }
}

pub struct ProcessExecInvokable {
    stdout: Option<mpsc::Receiver<Vec<u8>>>,
    stderr: Option<mpsc::Receiver<Vec<u8>>>,
    eval_rx: Option<mpsc::Receiver<EvalPlan>>,
    on_stdout: Option<WasmBusFeeder>,
    on_stderr: Option<WasmBusFeeder>,
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

pub struct ProcessExecSession {
    stdin: Option<mpsc::Sender<Vec<u8>>>,
}

impl Session for ProcessExecSession {
    fn call(&mut self, topic: &str, request: &Vec<u8>) -> Box<dyn Invokable + 'static> {
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
