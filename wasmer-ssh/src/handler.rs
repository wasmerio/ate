use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thrussh::CryptoVec;
use wasmer_wasi::os::Console;
use thrussh::server;
use thrussh::server::Auth;
use thrussh::server::Session;
use thrussh::ChannelId;
use thrussh_keys::key::ed25519;
use thrussh_keys::key::PublicKey;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use wasmer_wasi::os::InputEvent;
use wasmer_wasi::os::TtyOptions;
use wasmer_wasi::os::Tty;
use wasmer_wasi::wasmer_vbus::BusSpawnedProcessJoin;
use wasmer_wasi::WasiPipe;
use wasmer_wasi::WasiRuntimeImplementation;
use tokio::sync::mpsc;
use wasmer_wasi::bin_factory::CachedCompiledModules;
use wasmer_wasi::runtime::RuntimeStdout;

use super::runtime::*;
use super::error::*;

pub struct Handler {
    pub tty: Option<Tty>,
    pub peer_addr: Option<std::net::SocketAddr>,
    pub peer_addr_str: String,
    pub user: Option<String>,
    pub client_pubkey: Option<thrussh_keys::key::PublicKey>,
    pub console: Option<Console>,
    pub compiled_modules: Arc<CachedCompiledModules>,
}

impl Handler
{
    pub fn start_console(mut self, channel: ChannelId, session: Session, run: Option<String>) -> Pin<Box<dyn Future<Output = Result<(Self, Session), SshServerError>> + Send>>
    {
        // Create the handle
        let (tx_stdout, mut rx_stdout) = mpsc::channel(100);
        let (tx_stderr, mut rx_stderr) = mpsc::channel(100);
        let (tx_flush, mut rx_flush) = mpsc::channel(100);

        let user_agent = format!("ssh");
        let tty_options = TtyOptions::default();
        if run.is_some() {
            tty_options.set_echo(false);
            tty_options.set_line_buffering(false);
        }

        let runtime = Arc::new(SshRuntime {
            stdout: tx_stdout,
            stderr: tx_stderr,
            flush: tx_flush,
            tty: tty_options.clone(),
        });
        
        let (stdin_tx, stdin_rx) = WasiPipe::new();
        let tty = Tty::new(
            Box::new(stdin_tx),
            Box::new(RuntimeStdout::new(runtime.clone())),
            false,
            tty_options
        );
        self.tty.replace(tty);

        let mut console = Console::new(
            runtime.clone(),
            self.compiled_modules.clone(),
        );
        console = console
            .with_user_agent(user_agent.as_str())
            .with_stdin(stdin_rx);
        if let Some(run) = run {
            console = console.with_boot_cmd(run);
        }

        Box::pin(async move {
            // Add checkpoints
            let (checkpoint_tx, mut checkpoint_rx) = mpsc::channel(1);
            let (close_tx, close_rx) = tokio::sync::watch::channel(false);

            // Run the console (under a blocking thread)
            let (run_tx, mut run_rx) = tokio::sync::mpsc::channel(1);
            tokio::task::spawn_blocking(move || {
                let process = console.run();
                run_tx.blocking_send((console, process)).unwrap();
            });
            if let Some((console, process)) = run_rx.recv().await {
                self.console.replace(console);

                if let Ok(mut process) = process {
                    if let Some(tty) = self.tty.as_mut() {
                        if let Some(signaler) = process.signaler.take() {
                            tty.set_signaler(signaler)
                        }
                    }

                    let channel = channel.clone();
                    let mut handle = session.handle();
                    runtime.task_shared(Box::new(move || Box::pin(async move {
                        BusSpawnedProcessJoin::new(process).await;
                        
                        let _ = close_tx.send(true);
                        drop(close_tx);

                        while let Some(_) = checkpoint_rx.recv().await {
                            let _ = handle.flush(channel).await;
                        }
                        let _ = handle.close(channel).await;
                    }))).unwrap();
                } else {
                    let _ = close_tx.send(true);
                    drop(close_tx);
                }
            }

            // Create the handlers
            {
                let mut close_rx = close_rx.clone();
                let checkpoint_tx = checkpoint_tx.clone();
                let channel = channel.clone();
                let mut handle = session.handle();
                runtime.task_shared(Box::new(move || Box::pin(async move {
                    loop {
                        tokio::select! {
                            data = rx_stdout.recv() => {
                                if let Some(data) = data {
                                    let data = CryptoVec::from(data);
                                    let mut handle = handle.clone();
                                    let _ = handle.data(channel, data).await;
                                } else {
                                    break;
                                }
                            }
                            _ = close_rx.changed() => {
                                if *close_rx.borrow() == true {
                                    break;
                                }
                            }
                        }
                    }
                    while let Ok(data) = rx_stdout.try_recv() {
                        let data = CryptoVec::from(data);
                        let mut handle = handle.clone();
                        let _ = handle.data(channel, data).await;
                    }
                    let _ = handle.flush(channel).await;
                    let _ = checkpoint_tx.send(()).await;
                }))).unwrap();
            }
            {
                let mut close_rx = close_rx.clone();
                let checkpoint_tx = checkpoint_tx.clone();
                let channel = channel.clone();
                let mut handle = session.handle();
                runtime.task_shared(Box::new(move || Box::pin(async move {
                    loop {
                        tokio::select! {
                            data = rx_stderr.recv() => {
                                if let Some(data) = data {
                                    let data = CryptoVec::from(data);
                                    let mut handle = handle.clone();
                                    let _ = handle.extended_data(channel, 1, data).await;
                                } else {
                                    break;
                                }
                            }
                            _ = close_rx.changed() => {
                                if *close_rx.borrow() == true {
                                    break;
                                }
                            }
                        }
                    }
                    while let Ok(data) = rx_stderr.try_recv() {
                        let data = CryptoVec::from(data);
                        let mut handle = handle.clone();
                        let _ = handle.extended_data(channel, 1, data).await;
                    }
                    let _ = handle.flush(channel).await;
                    let _ = checkpoint_tx.send(()).await;
                }))).unwrap();
            }
            {
                let channel = channel.clone();
                let handle = session.handle();
                runtime.task_shared(Box::new(move || Box::pin(async move {
                    while let Some(_) = rx_flush.recv().await {
                        let mut handle = handle.clone();
                        let _ = handle.flush(channel).await;
                    }
                }))).unwrap();
            }

            // We are ready to receive data
            Ok((self, session))
        })
    }
}

impl server::Handler for Handler {
    type Error = SshServerError;
    type FutureAuth = Pin<Box<dyn Future<Output = Result<(Self, Auth), Self::Error>> + Send>>;
    type FutureUnit = Pin<Box<dyn Future<Output = Result<(Self, Session), Self::Error>> + Send>>;
    type FutureBool =
        Pin<Box<dyn Future<Output = Result<(Self, Session, bool), Self::Error>> + Send>>;

    fn finished_auth(self, auth: Auth) -> Self::FutureAuth {
        Box::pin(async move { Ok((self, auth)) })
    }

    fn finished_bool(self, b: bool, session: Session) -> Self::FutureBool {
        Box::pin(async move { Ok((self, session, b)) })
    }

    fn finished(self, session: Session) -> Self::FutureUnit {
        Box::pin(async move { Ok((self, session)) })
    }

    fn auth_keyboard_interactive(
        mut self,
        user: &str,
        _submethods: &str,
        response: Option<server::Response>,
    ) -> Self::FutureAuth {
        debug!("authenticate with keyboard interactive (user={})", user);
        self.user = Some(user.to_string());

        /*
        // Root is always rejected (as this is what bots attack on)
        if user == "root" {
            warn!("root attempt rejected from {}", self.peer_addr_str);
            wizard.fail("root not supported - instead use 'ssh joe@blogs.com@wasmer.sh'\r\n");
        }
        */

        // Process it in the wizard
        let _response = match response {
            Some(mut a) => Some(convert_response(&mut a)),
            None => None,
        };

        // Unfortunately the SSH server isnt working properly so we accept
        // the session into the shell and process it there instead
        self.finished_auth(Auth::Accept)
    }

    fn data(mut self, channel: ChannelId, data: &[u8], session: Session) -> Self::FutureUnit {
        trace!("data on channel {:?}: len={:?}", channel, data.len());
        
        let data = data.to_vec();
        Box::pin(async move {
            if let Some(tty) = self.tty.as_mut() {
                tty.on_event(InputEvent::Data(unsafe {
                    String::from_utf8_unchecked(data)
                }));
            }
            Ok((self, session))
        })
    }

    fn exec_request(self, channel: ChannelId, data: &[u8], session: Session) -> Self::FutureUnit {
        let cmd = String::from_utf8_lossy(data);
        debug!("exec_request: {})", cmd);

        self.start_console(channel, session, Some(cmd.to_string()))
    }

    fn shell_request(self, channel: ChannelId, session: Session) -> Self::FutureUnit {
        debug!("shell_request");

        self.start_console(channel, session, None)
    }

    #[allow(unused_variables)]
    fn channel_close(self, channel: ChannelId, session: Session) -> Self::FutureUnit {
        debug!("channel_close");

        self.finished(session)
    }

    #[allow(unused_variables)]
    fn channel_eof(mut self, channel: ChannelId, session: Session) -> Self::FutureUnit {
        debug!("channel_eof");

        Box::pin(async move {
            self.tty.take();
            Ok((self, session))
        })
    }

    #[allow(unused_variables)]
    fn pty_request(
        self,
        channel: ChannelId,
        term: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        modes: &[(thrussh::Pty, u32)],
        session: Session,
    ) -> Self::FutureUnit {
        debug!("pty_request");

        if let Some(tty) = self.tty.as_ref() {
            let tty = tty.options();
            tty.set_cols(col_width);
            tty.set_rows(row_height);
            //self.rect.width = pix_width;
            //self.rect.height = pix_height;
            tty.set_echo(modes.iter().any(|(k, v)| *k == thrussh::Pty::ECHO && *v == 1));
            tty.set_line_buffering(modes.iter().any(|(k, v)| *k == thrussh::Pty::ICANON && *v == 1));
        }

        self.finished(session)
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        info!("ssh connection closed ({})", self.peer_addr_str);
    }
}

#[allow(dead_code)]
fn clone_public_key(key: &PublicKey) -> PublicKey {
    match key {
        PublicKey::Ed25519(a) => PublicKey::Ed25519(ed25519::PublicKey { key: a.key.clone() }),
    }
}

fn convert_response<'a>(response: &mut thrussh::server::Response<'a>) -> Vec<String> {
    let mut ret = Vec::new();

    for txt in response.map(|a| a.to_vec()).collect::<Vec<Vec<u8>>>() {
        if let Ok(txt) = String::from_utf8(txt) {
            ret.push(txt);
        } else {
            break;
        }
    }
    ret
}
