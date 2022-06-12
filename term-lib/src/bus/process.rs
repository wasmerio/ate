use async_trait::async_trait;
use derivative::*;
use std::any::type_name;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus::abi::SerializationFormat;
use wasm_bus_process::api;
use wasm_bus_process::prelude::*;

use super::*;
use crate::api::*;
use crate::err;
use crate::eval::*;
use crate::fd::*;
use crate::stdout::*;
use crate::pipe::*;
use crate::reactor::*;

pub struct EvalCreated {
    pub invoker: ProcessExecInvokable,
    pub session: ProcessExecSession,
}

struct ProcessExecCreate {
    request: api::PoolSpawnRequest,
    on_stdout: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
    on_stderr: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
    on_exit: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ProcessExecFactory {
    system: System,
    compiler: Compiler,
    #[derivative(Debug = "ignore")]
    reactor: Arc<RwLock<Reactor>>,
    #[derivative(Debug = "ignore")]
    pub(crate) exec_factory: EvalFactory,
    #[derivative(Debug = "ignore")]
    pub(crate) ctx: Arc<Mutex<Option<EvalContext>>>,
}

pub struct EvalContextTaker {
    ctx: Arc<Mutex<Option<EvalContext>>>,
}

impl EvalContextTaker {
    pub fn new(factory: &ProcessExecFactory) -> EvalContextTaker {
        EvalContextTaker {
            ctx: factory.ctx.clone()
        }
    }
    
    pub fn take_context(&self) -> Option<EvalContext> {
        let mut guard = self.ctx.lock().unwrap();
        guard.take()
    }
}

pub struct LaunchContext {
    eval: EvalContext,
    path: String,
    args: Vec<String>,
    stdin_tx: Option<mpsc::Sender<FdMsg>>,
    stdout_rx: Option<mpsc::Receiver<FdMsg>>,
    stderr_rx: Option<mpsc::Receiver<FdMsg>>,
    on_stdout: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
    on_stderr: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
    on_exit: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct LaunchEnvironment {
    #[derivative(Debug = "ignore")]
    pub abi: Arc<dyn ConsoleAbi>,
    pub inherit_stdin: WeakFd,
    pub inherit_stdout: WeakFd,
    pub inherit_stderr: WeakFd,
    pub inherit_log: WeakFd,
}

impl ProcessExecFactory {
    pub fn new(
        reactor: Arc<RwLock<Reactor>>,
        compiler: Compiler,
        exec_factory: EvalFactory,
        ctx: EvalContext,
    ) -> ProcessExecFactory {
        let system = System::default();
        ProcessExecFactory {
            system,
            reactor,
            compiler,
            exec_factory,
            ctx: Arc::new(Mutex::new(Some(ctx))),
        }
    }

    pub async fn launch<T, F>(
        &self,
        request: api::PoolSpawnRequest,
        env: &LaunchEnvironment,
        mut client_callbacks: HashMap<String, Arc<dyn BusFeeder + Send + Sync + 'static>>,
        funct: F,
    ) -> Result<T, BusError>
    where
        F: Fn(LaunchContext) -> Pin<Box<dyn Future<Output = Result<T, u32>>>>,
        F: Send + 'static,
        T: Send,
    {
        // Grab the callbacks and build the requiest
        let on_stdout =
            client_callbacks.remove(&type_name::<api::PoolSpawnStdoutCallback>().to_string());
        let on_stderr =
            client_callbacks.remove(&type_name::<api::PoolSpawnStderrCallback>().to_string());
        let on_exit =
            client_callbacks.remove(&type_name::<api::PoolSpawnExitCallback>().to_string());

        let create = ProcessExecCreate {
            request,
            on_stdout,
            on_stderr,
            on_exit,
        };

        // Push all the cloned variables into a background thread so
        // that it does not hurt anything
        let abi = env.abi.clone();
        let ctx = self.ctx.clone();
        let reactor = self.reactor.clone();
        let compiler = self.compiler;
        let inherit_stdin = env.inherit_stdin.upgrade();
        let inherit_stdout = env.inherit_stdout.upgrade();
        let inherit_stderr = env.inherit_stderr.upgrade();
        let inherit_log = env.inherit_log.upgrade();
        let exec_factory = self.exec_factory.clone();
        let result = self.system.spawn_dedicated(move || async move {
            let path = create.request.spawn.path;
            let args = create.request.spawn.args;
            let chroot = create.request.spawn.chroot;
            let working_dir = create.request.spawn.working_dir;
            let stdin_mode = create.request.spawn.stdin_mode;
            let stdout_mode = create.request.spawn.stdout_mode;
            let stderr_mode = create.request.spawn.stderr_mode;
            let pre_open = create.request.spawn.pre_open;
            let on_stdout = create.on_stdout;
            let on_stderr = create.on_stderr;
            let on_exit = create.on_exit;

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
            let (stdin, stdin_tx) = pipe_in(ReceiverMode::Stream, FdFlag::Stdin(false));
            let (stdout, stdout_rx) = pipe_out(FdFlag::Stdout(false));
            let (stderr, stderr_rx) = pipe_out(FdFlag::Stderr(false));

            // Perform hooks back to the main stdio
            let (stdin, stdin_tx) = match stdin_mode {
                StdioMode::Null => (stdin, None),
                StdioMode::Inherit if inherit_stdin.is_some() => {
                    (inherit_stdin.clone().unwrap(), None)
                }
                StdioMode::Inherit => (stdin, None),
                StdioMode::Piped => (stdin, Some(stdin_tx)),
                StdioMode::Log => (stdin, None),
            };
            let (stdout, stdout_rx) = match stdout_mode {
                StdioMode::Null => (stdout, None),
                StdioMode::Inherit if inherit_stdout.is_some() => {
                    (inherit_stdout.clone().unwrap(), None)
                }
                StdioMode::Inherit => (stdout, None),
                StdioMode::Piped => (stdout, Some(stdout_rx)),
                StdioMode::Log if inherit_log.is_some() => (inherit_log.clone().unwrap(), None),
                StdioMode::Log if inherit_stdout.is_some() => {
                    (inherit_stdout.clone().unwrap(), None)
                }
                StdioMode::Log => (stdout, None),
            };
            let (stderr, stderr_rx) = match stderr_mode {
                StdioMode::Null => (stderr, None),
                StdioMode::Inherit if inherit_stderr.is_some() => (inherit_stderr.unwrap(), None),
                StdioMode::Inherit => (stderr, None),
                StdioMode::Piped => (stderr, Some(stderr_rx)),
                StdioMode::Log if inherit_log.is_some() => (inherit_log.clone().unwrap(), None),
                StdioMode::Log if inherit_stderr.is_some() => {
                    (inherit_stderr.clone().unwrap(), None)
                }
                StdioMode::Log => (stderr, None),
            };

            // Create the eval context
            let spawn = {
                let guard = ctx.lock().unwrap();
                let ctx = match guard.as_ref() {
                    Some(a) => a,
                    None => {
                        error!(
                            "The eval context has been lost has sub-processes can not be started."
                        );
                        return Err(err::ERR_ENOEXEC);
                    }
                };
                SpawnContext::new(
                    abi,
                    ctx.env.clone(),
                    job.clone(),
                    stdin,
                    stdout,
                    stderr,
                    chroot,
                    working_dir
                        .as_ref()
                        .map(|a| a.clone())
                        .unwrap_or(ctx.working_dir.clone()),
                    pre_open,
                    ctx.root.clone(),
                    compiler,
                )
            };
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
                on_exit,
            };

            // Start the process
            Ok(funct(ctx).await?)
        });

        let ret = match result.await.ok_or_else(|| BusError::Aborted)? {
            Ok(created) => created,
            Err(err) => {
                let err: u32 = err;
                warn!("failed to create process - internal error - code={}", err);
                return Err(BusError::Unknown);
            }
        };

        Ok(ret)
    }

    pub async fn eval(
        &self,
        request: api::PoolSpawnRequest,
        env: &LaunchEnvironment,
        this_callback: Arc<dyn BusFeeder + Send + Sync + 'static>,
        client_callbacks: HashMap<String, Arc<dyn BusFeeder + Send + Sync + 'static>>,
    ) -> Result<EvalCreated, BusError> {
        let dst = Arc::clone(&self.ctx);
        self.launch(request, env, client_callbacks, move |ctx: LaunchContext| {
            let dst = dst.clone();
            let this_callback = this_callback.clone();
            Box::pin(async move {
                let cmd = ctx.path.clone();
                let eval_rx = crate::eval::eval(cmd, ctx.eval);
                let on_ctx = Box::pin(move |src: EvalContext| {
                    let mut guard = dst.lock().unwrap();
                    if let Some(dst) = guard.as_mut() {
                        dst.env = src.env;
                        dst.root = src.root;
                        dst.working_dir = src.working_dir;
                    }
                });

                Ok(EvalCreated {
                    invoker: ProcessExecInvokable {
                        exec: Some(ProcessExec {
                            format: SerializationFormat::Bincode,
                            stdout: ctx.stdout_rx,
                            stderr: ctx.stderr_rx,
                            eval_rx,
                            this: this_callback,
                            on_stdout: ctx.on_stdout,
                            on_stderr: ctx.on_stderr,
                            on_exit: ctx.on_exit,
                            on_ctx,
                        }),
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
        request: api::PoolSpawnRequest,
        env: &LaunchEnvironment,
        client_callbacks: HashMap<String, Arc<dyn BusFeeder + Send + Sync + 'static>>,
    ) -> Result<
        (
            Process,
            AsyncResult<(EvalContext, u32)>,
            Arc<WasmBusThreadPool>,
        ),
        BusError,
    > {
        self.launch(request, env, client_callbacks, |ctx: LaunchContext| {
            Box::pin(async move {
                let stdio = ctx.eval.stdio.clone();
                let env = ctx.eval.env.clone().into_exported();

                let mut show_result = false;
                let redirect = Vec::new();

                let ret = exec_process(
                    ctx.eval,
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

    pub fn take_context(&self) -> Option<EvalContext> {
        let mut guard = self.ctx.lock().unwrap();
        guard.take()
    }

    pub fn stdio(&self, env: &LaunchEnvironment) -> crate::stdio::Stdio {
        let mut stdio = self.exec_factory.stdio(self.stdin(env));
        stdio.stdin = self.stdin(env);
        stdio.stdout = self.stdout(env).fd();
        stdio.stderr = self.stderr(env);
        stdio
    }

    pub fn stdin(&self, env: &LaunchEnvironment) -> Fd {
        use crate::pipe::*;

        if let Some(fd) = env.inherit_stdin.upgrade() {
            fd
        } else {
            let (stdin_fd, _) = pipe_in(ReceiverMode::Stream, FdFlag::Stdin(false));
            stdin_fd
        }
    }

    pub fn stdout(&self, env: &LaunchEnvironment) -> Stdout {
        if let Some(fd) = env.inherit_stdout.upgrade() {
            Stdout::new(fd)
        } else {
            self.exec_factory.stdout()
        }        
    }

    pub fn stderr(&self, env: &LaunchEnvironment) -> Fd {
        if let Some(fd) = env.inherit_stderr.upgrade() {
            fd
        } else {
            self.exec_factory.stderr()
        }        
    }
}

pub struct ProcessExec {
    format: SerializationFormat,
    stdout: Option<mpsc::Receiver<FdMsg>>,
    stderr: Option<mpsc::Receiver<FdMsg>>,
    eval_rx: mpsc::Receiver<EvalResult>,
    on_stdout: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
    on_stderr: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
    on_exit: Option<Arc<dyn BusFeeder + Send + Sync + 'static>>,
    on_ctx: Pin<Box<dyn Fn(EvalContext) + Send + 'static>>,
    this: Arc<dyn BusFeeder + Send + Sync + 'static>,
}

impl ProcessExec {
    pub async fn run(mut self) {
        use std::ops::Deref;

        // Now process all the STDIO concurrently
        loop {
            if let Some(stdout_rx) = self.stdout.as_mut() {
                if let Some(stderr_rx) = self.stderr.as_mut() {
                    tokio::select! {
                        msg = stdout_rx.recv() => {
                            if let (Some(msg), Some(on_data)) = (msg, self.on_stdout.as_ref()) {
                                if let FdMsg::Data { data, .. } = msg {
                                    BusFeederUtils::feed(on_data.deref(), self.format, api::PoolSpawnStdoutCallback(data));
                                }
                            } else {
                                self.stdout.take();
                            }
                        }
                        msg = stderr_rx.recv() => {
                            if let (Some(msg), Some(on_data)) = (msg, self.on_stderr.as_ref()) {
                                if let FdMsg::Data { data, .. } = msg {
                                    BusFeederUtils::feed(on_data.deref(), self.format, api::PoolSpawnStderrCallback(data));
                                }
                            } else {
                                self.stderr.take();
                            }
                        }
                        res = self.eval_rx.recv() => {
                            let res = encode_eval_response(self.format, self.on_ctx, res);
                            if let Some(on_exit) = self.on_exit.take() {
                                BusFeederUtils::feed_bytes_or_error(on_exit.deref(), res);
                            }
                            break;
                        }
                    }
                } else {
                    tokio::select! {
                        msg = stdout_rx.recv() => {
                            if let (Some(msg), Some(on_data)) = (msg, self.on_stdout.as_ref()) {
                                if let FdMsg::Data { data, .. } = msg {
                                    BusFeederUtils::feed(on_data.deref(), self.format, api::PoolSpawnStdoutCallback(data));
                                }
                            } else {
                                self.stdout.take();
                            }
                        }
                        res = self.eval_rx.recv() => {
                            let res = encode_eval_response(self.format, self.on_ctx, res);
                            if let Some(on_exit) = self.on_exit.take() {
                                BusFeederUtils::feed_bytes_or_error(on_exit.deref(), res);
                            }
                            break;
                        }
                    }
                }
            } else {
                if let Some(stderr_rx) = self.stderr.as_mut() {
                    tokio::select! {
                        msg = stderr_rx.recv() => {
                            if let (Some(msg), Some(on_data)) = (msg, self.on_stderr.as_ref()) {
                                if let FdMsg::Data { data, .. } = msg {
                                    BusFeederUtils::feed(on_data.deref(), self.format, api::PoolSpawnStderrCallback(data));
                                }
                            } else {
                                self.stderr.take();
                            }
                        }
                        res = self.eval_rx.recv() => {
                            let res = encode_eval_response(self.format, self.on_ctx, res);
                            if let Some(on_exit) = self.on_exit.take() {
                                BusFeederUtils::feed_bytes_or_error(on_exit.deref(), res);
                            }
                            break;
                        }
                    }
                } else {
                    tokio::select! {
                        res = self.eval_rx.recv() => {
                            let res = encode_eval_response(self.format, self.on_ctx, res);
                            if let Some(on_exit) = self.on_exit.take() {
                                BusFeederUtils::feed_bytes_or_error(on_exit.deref(), res);
                            }
                            break;
                        }
                    }
                }
            }
        }
        self.this.terminate();
    }
}

pub struct ProcessExecInvokable {
    exec: Option<ProcessExec>,
}

#[async_trait]
impl Invokable for ProcessExecInvokable {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        let exec = self.exec.take();
        if let Some(exec) = exec {
            let fut = Box::pin(exec.run());
            Ok(InvokeResult::ResponseThenWork(
                encode_response(SerializationFormat::Bincode, &())?,
                fut,
            ))
        } else {
            Err(BusError::Unknown)
        }
    }
}

fn encode_eval_response(
    format: SerializationFormat,
    on_ctx: Pin<Box<dyn Fn(EvalContext) + Send + 'static>>,
    res: Option<EvalResult>,
) -> Result<Vec<u8>, BusError> {
    match res {
        Some(res) => {
            {
                let on_ctx = on_ctx.as_ref();
                on_ctx(res.ctx);
            }
            Ok(encode_response(
                format,
                &match res.status {
                    EvalStatus::Executed { code, .. } => api::PoolSpawnExitCallback(code as i32),
                    EvalStatus::InternalError => {
                        api::PoolSpawnExitCallback(err::ERR_ENOEXEC as i32)
                    }
                    EvalStatus::Invalid => api::PoolSpawnExitCallback(err::ERR_EINVAL as i32),
                    EvalStatus::MoreInput => api::PoolSpawnExitCallback(err::ERR_EINVAL as i32),
                },
            )?)
        }
        None => Ok(encode_response(
            format,
            &api::PoolSpawnExitCallback(err::ERR_EINVAL as i32),
        )?),
    }
}

#[derive(Clone)]
pub struct ProcessExecSession {
    stdin: Option<mpsc::Sender<FdMsg>>,
}

impl Session for ProcessExecSession {
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        let ret = {
            if topic == type_name::<api::ProcessStdinRequest>() {
                let request: api::ProcessStdinRequest =
                    match decode_request(SerializationFormat::Bincode, request.as_ref()) {
                        Ok(a) => a,
                        Err(err) => {
                            return Ok((ErrornousInvokable::new(err), None));
                        }
                    };
                if let Some(stdin) = self.stdin.as_ref() {
                    let tx_send = stdin.clone();
                    let _ = tx_send.blocking_send(FdMsg::new(request.data, FdFlag::Stdin(false)));
                }
                ResultInvokable::new(SerializationFormat::Bincode, ())
            } else if topic == type_name::<api::ProcessCloseStdinRequest>() {
                let _request: api::ProcessCloseStdinRequest =
                    match decode_request(SerializationFormat::Bincode, request.as_ref()) {
                        Ok(a) => a,
                        Err(err) => {
                            return Ok((ErrornousInvokable::new(err), None));
                        }
                    };
                self.stdin.take();
                ResultInvokable::new(SerializationFormat::Bincode, ())
            } else {
                ErrornousInvokable::new(BusError::InvalidTopic)
            }
        };
        Ok((ret, None))
    }
}
