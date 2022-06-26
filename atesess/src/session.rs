use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use tokio::sync::Mutex as AsyncMutex;
use ate::prelude::*;
use ate::comms::*;
use tokera::model::InstanceCall;
use tokera::model::InstanceCommand;
use tokera::model::InstanceHello;
use tokera::model::InstanceReply;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use atessh::term_lib;
use term_lib::console::Console;
use tokio::sync::mpsc;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use term_lib::api::ConsoleRect;
use term_lib::bus;
use term_lib::bus::*;
use term_lib::eval::EvalStatus;
use term_lib::environment::Environment;
use term_lib::api::System;
use term_lib::api::SystemAbiExt;
use term_lib::common::MAX_MPSC;
use term_lib::fd::WeakFd;
use term_lib::api::AsyncResult;
use term_lib::fd::Fd;
use term_lib::grammar::ast::Redirect;

use super::handler::SessionHandler;
use super::handler::SessionTx;
use super::server::SessionBasics;

pub struct Session
{
    pub rx: Box<dyn StreamReadable + Send + Sync + 'static>,
    pub tx: Option<Upstream>,
    pub hello: HelloMetadata,
    pub hello_instance: InstanceHello,
    pub sock_addr: SocketAddr,
    pub wire_encryption: Option<EncryptKey>,
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub compiler: term_lib::eval::Compiler,
    pub handler: Arc<SessionHandler>,
    pub console: Console,
    pub exit_rx: mpsc::Receiver<()>,
    pub factories: HashMap<String, BusFactory>,
    pub basics: SessionBasics,
    pub invocations: SessionInvocations,
}

impl Session
{
    pub async fn new(
        rx: Box<dyn StreamReadable + Send + Sync + 'static>,
        tx: Option<Upstream>,
        hello: HelloMetadata,
        hello_instance: InstanceHello,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
        rect: Arc<Mutex<ConsoleRect>>,
        compiler: term_lib::eval::Compiler,
        basics: SessionBasics,
        first_init: bool,
    ) -> Session
    {
        // Create the handler
        let (exit_tx, exit_rx) = mpsc::channel(1);
        let handler = SessionHandler {
            tx: AsyncMutex::new(SessionTx::None),
            rect: rect.clone(),
            exit: exit_tx,
        };
        let handler = Arc::new(handler);

        // Create the console
        let id_str = basics.service_instance.id_str();
        let prompt = (&id_str[0..9]).to_string();
        let location = format!("ssh://tokera.sh/?no_welcome&prompt={}", prompt);
        let user_agent = "noagent".to_string();
        let mut console = Console::new_ext(
            location,
            user_agent,
            compiler,
            handler.clone(),
            None,
            basics.fs.clone(),
            basics.bins.clone(),
            basics.reactor.clone(),
        );

        // If its the first init
        if first_init {
            console.init().await;
        }

        // We prepare the console which runs a few initialization steps
        // that are needed before anything is invoked
        console.prepare().await;

        // Return the session
        Session {
            rx,
            tx,
            hello,
            hello_instance,
            sock_addr,
            wire_encryption,
            rect,
            compiler,
            exit_rx,
            handler,
            console,
            factories: HashMap::default(),
            basics,
            invocations: SessionInvocations::default(),
        }
    }

    pub async fn get_or_create_factory(&mut self, cmd: String) -> Option<&mut BusFactory>
    {
        // Check for existing
        if self.factories.contains_key(&cmd) {
            return Some(self.factories.get_mut(&cmd).unwrap());
        }

        // Create the job and context
        let exec_factory = self.console.exec_factory();
        let job = self.console.new_job().await?;
        let ctx = exec_factory.create_context(self.console.new_spawn_context(&job));
        let multiplexer = self.basics.multiplexer.clone();

        // Create the process factory that used by this process to create sub-processes
        let sub_process_factory = ProcessExecFactory::new(
            self.console.reactor(),
            self.compiler,
            exec_factory,
            ctx,
        );
        let bus_factory = BusFactory::new(sub_process_factory, multiplexer);

        // Add the factory then return it
        self.factories.insert(cmd.clone(), bus_factory);
        return Some(self.factories.get_mut(&cmd).unwrap());
    }

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        // Start a queue that will send data back to the client
        let (tx_reply, mut rx_reply) = mpsc::channel(MAX_MPSC);
        let mut is_feeder_set = false;

        // Wait for commands to come in and then process them
        loop {
            let invocations = self.invocations.clone();
            tokio::select! {
                cmd = self.rx.read() => {
                    let cmd = cmd?;
                    debug!("session read (len={})", cmd.len());

                    let action: InstanceCommand = serde_json::from_slice(&cmd[..])?;
                    debug!("session cmd ({})", action);

                    match action {
                        InstanceCommand::Shell => {
                            return self.shell().await;
                        }
                        InstanceCommand::Call(call) => {
                            // Set the handler to use the upstream via instance reply messages
                            if is_feeder_set == false {
                                is_feeder_set = true;
                                let mut guard = self.handler.tx.lock().await;
                                *guard = SessionTx::Feeder(tx_reply.clone());
                            }

                            // Invoke the call
                            let req = self.rx.read().await?;
                            self.call(call, req, tx_reply.clone()).await?;
                        }
                    }
                }
                reply = rx_reply.recv() => {
                    if let Some(reply) = reply {
                        let reply = bincode::serialize(&reply).unwrap();
                        if let Some(tx) = self.tx.as_mut() {
                            let _ = tx.outbox.write(&reply[..]).await;
                        }
                    }
                }
                _ = invocations => {
                }
            }
        }
    }

    pub async fn shell(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("new connection from {}", self.sock_addr);

        // Check the access code matches what was passed in
        if self.hello_instance.access_token.eq_ignore_ascii_case(self.basics.service_instance.admin_token.as_str()) == false {
            warn!("access denied to {} from {}", self.hello_instance.chain, self.sock_addr);
            let err: CommsError = CommsErrorKind::FatalError("access denied".to_string()).into();
            return Err(err.into());
        }

        // Set the handler to use the upstream tx directly
        {
            let mut guard = self.handler.tx.lock().await;
            *guard = match self.tx {
                Some(tx) => SessionTx::Upstream(tx),
                None => SessionTx::None,
            };
        }

        // Draw the prompt
        self.console.tty_mut().draw_prompt().await;

        // Enter a processing loop
        loop {
            let invocations = self.invocations.clone();
            tokio::select! {
                data = self.rx.read() => {
                    match data {
                        Ok(data) => {
                            let data = String::from_utf8_lossy(&data[..]);
                            self.console.on_data(data.into()).await;
                        }
                        Err(err) => {
                            info!("exiting from session ({}) - {}", self.basics.service_instance.id_str(), err);
                            break;        
                        }
                    }
                },
                _ = self.exit_rx.recv() => {
                    info!("exiting from session ({})", self.basics.service_instance.id_str());
                    break;
                }
                _ = invocations => {
                }
            }
        }
        Ok(())
    }

    pub async fn eval(&mut self, binary: String, env: Environment, args: Vec<String>, redirects: Vec<Redirect>, stdin: Fd, stdout: Fd, stderr: Fd) -> Result<u32, Box<dyn std::error::Error>>
    {
        // Build the job and the environment
        let job = self.console.new_job()
            .await
            .ok_or_else(|| {
                let err: CommsError = CommsErrorKind::FatalError("no more job space".to_string()).into();
                err
            })?;

        let mut ctx = self.console.new_spawn_context(&job);
        ctx.stdin = stdin;
        ctx.stdout = stdout;
        ctx.stderr = stderr;
        ctx.env = env;
        ctx.extra_args = args;
        ctx.extra_redirects = redirects;
        let exec = self.console.exec_factory();

        // Execute the binary
        let mut eval = exec.eval(binary, ctx);

        // Get the result and return the status code
        let ret = eval.recv().await;
        let ret = match ret {
            Some(ret) => {
                match ret.status {
                    EvalStatus::Executed { code, .. } => code,
                    _ => 1,
                }
            }
            None => {
                let err: CommsError = CommsErrorKind::FatalError("failed to evaluate binary".to_string()).into();
                return Err(err.into());
            }
        };
        Ok(ret)
    }

    pub async fn can_access_binary(&self, binary: &str, access_token: &str) -> bool
    {
        // Check the access code matches what was passed in
        self.basics
            .service_instance
            .exports
            .iter()
            .await
            .map_or(false, |iter| {
                iter
                    .filter(|e| e.binary.eq_ignore_ascii_case(binary))
                    .any(|e| e.access_token.eq_ignore_ascii_case(access_token))
            })
        
    }

    pub async fn call(&mut self, call: InstanceCall, request: Vec<u8>, tx_reply: mpsc::Sender<InstanceReply>) -> Result<(), Box<dyn std::error::Error>>
    {
        // Create the callbacks
        let this_callback = SessionFeeder {
            sys: System::default(),
            tx_reply,
            handle: call.handle.into(),
        };
        let feeder = this_callback.clone();
        let client_callbacks = HashMap::default();

        // Check the access code matches what was passed in
        if self.basics.service_instance
            .exports
            .iter()
            .await?
            .filter(|e| e.binary.eq_ignore_ascii_case(call.binary.as_str()))
            .any(|e| e.access_token.eq_ignore_ascii_case(self.hello_instance.access_token.as_str()))
            == false
        {
            warn!("access denied to {}@{} from {}", call.binary, self.hello_instance.chain, self.sock_addr);
            this_callback.error(BusError::AccessDenied);
            return Ok(());
        }

        // Create the context
        let caller_ctx = WasmCallerContext::default();

        // Create the job and context
        let launch_env = LaunchEnvironment {
            abi: self.console.abi(),
            inherit_stdin: WeakFd::null(),
            inherit_stderr: self.console.stderr_fd().downgrade(),
            inherit_stdout: self.console.stdout_fd().downgrade(),
            inherit_log: self.console.stderr_fd().downgrade(),
        };
        
        // Invoke a call with using the console object
        let bus_factory = self.get_or_create_factory(call.binary.clone())
            .await
            .ok_or_else(|| {
                let err: CommsError = CommsErrorKind::FatalError("bus factory error - failed to create factory".to_string()).into();
                err
            })?;

        // Now invoke it on the bus factory
        let mut invoke = bus_factory.start(
            call.parent.map(|a| a.into()),
            call.handle.into(),
            call.binary,
            call.topic,
            request,
            Arc::new(this_callback),
            client_callbacks,
            caller_ctx.clone(),
            call.keepalive,
            launch_env
        );

        // Invoke the send operation
        let sys = System::default();
        let (abort_tx, mut abort_rx) = mpsc::channel(1);
        let result = {
            sys.spawn_shared(move || async move {
                tokio::select! {
                    response = invoke.process() => {
                        response
                    }
                    _ = abort_rx.recv() => {
                        Err(BusError::Aborted)
                    }
                }
            })
        };

        // Add the invocation
        let sessions = bus_factory.sessions();
        self.invocations.push(SessionInvocation {
            feeder,
            result,
            sessions,
            _abort_tx: abort_tx,
        });
        Ok(())
    }
}

pub struct SessionInvocation
{
    feeder: SessionFeeder,
    result: AsyncResult<Result<InvokeResult, BusError>>,
    sessions: Arc<Mutex<HashMap<CallHandle, Box<dyn bus::Session>>>>,
    _abort_tx: mpsc::Sender<()>,
}

#[derive(Default, Clone)]
pub struct SessionInvocations {
    pub running: Arc<Mutex<Vec<SessionInvocation>>>,
}

impl SessionInvocations {
    pub fn push(&self, invoke: SessionInvocation) {
        let mut running = self.running.lock().unwrap();
        running.push(invoke);
    }
}

impl Future for SessionInvocations {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut running = self.running.lock().unwrap();

        let mut carry = Vec::new();
        for mut invoke in running.drain(..) {
            let mut rx = Pin::new(&mut invoke.result.rx);
            match rx.poll_recv(cx) {
                Poll::Ready(Some(result)) => {
                    BusFeederUtils::process(&invoke.feeder, result, &invoke.sessions);
                }
                Poll::Ready(None) => {
                    BusFeederUtils::process(&invoke.feeder, Err(BusError::Aborted), &invoke.sessions);
                }
                Poll::Pending => {
                    carry.push(invoke);
                }
            }
        }

        running.append(&mut carry);

        Poll::Pending
    }
}

#[derive(Debug, Clone)]
pub struct SessionFeeder {
    sys: System,
    tx_reply: mpsc::Sender<InstanceReply>,
    handle: CallHandle,
}

impl SessionFeeder {
    fn send(&self, reply: InstanceReply) {
        self.sys.fire_and_forget(&self.tx_reply, reply);
    }
}

impl BusStatelessFeeder
for SessionFeeder {
    fn feed_bytes(&self, data: Vec<u8>) {
        trace!("feed-bytes(handle={}, data={} bytes)", self.handle, data.len());
        self.send(InstanceReply::FeedBytes {
            handle: self.handle,
            data
        });
    }

    fn error(&self, err: BusError) {
        trace!("error(handle={}, err={})", self.handle, err);
        self.send(InstanceReply::Error {
            handle: self.handle,
            error: err
        });
    }

    fn terminate(&self) {
        trace!("terminate(handle={})", self.handle);
        self.send(InstanceReply::Terminate {
            handle: self.handle,
        });
    }
}

impl BusStatefulFeeder
for SessionFeeder {
    fn handle(&self) -> CallHandle {
        self.handle
    }
}