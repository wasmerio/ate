use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use wasmer_os::api::ConsoleRect;
use wasmer_os::api::System;
use wasmer_os::console::Console;
use thrussh::server;
use thrussh::server::Auth;
use thrussh::server::Session;
use thrussh::ChannelId;
use thrussh_keys::key::ed25519;
use thrussh_keys::key::PublicKey;
use wasmer_term::wasmer_os;
use wasmer_term::wasmer_os::api as term_api;
use wasmer_term::wasmer_os::api::SystemAbiExt;
use wasmer_term::wasmer_os::bin_factory::CachedCompiledModules;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::wizard::SshWizard;

use super::console_handle::*;
use super::error::*;

pub struct Handler {
    pub peer_addr: Option<std::net::SocketAddr>,
    pub peer_addr_str: String,
    pub user: Option<String>,
    pub client_pubkey: Option<thrussh_keys::key::PublicKey>,
    pub console: Option<Console>,
    pub engine: Option<wasmer_os::wasmer::Engine>,
    pub compiler: wasmer_os::eval::Compiler,
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub wizard: Option<SshWizard>,
    pub compiled_modules: Arc<CachedCompiledModules>,
    pub webc_dir: Option<String>,
    pub stdio_lock: Arc<Mutex<()>>,
}

impl Handler
{
    pub fn start_console(mut self, channel: ChannelId, session: Session, run: Option<String>) -> Pin<Box<dyn Future<Output = Result<(Self, Session), SshServerError>> + Send>>
    {
        Box::pin(async move {
            // Create the handle
            let handle = Arc::new(ConsoleHandle {
                rect: self.rect.clone(),
                channel: channel.clone(),
                handle: session.handle(),
                stdio_lock: self.stdio_lock.clone(),
                enable_stderr: true,
            });

            // Spawn a dedicated thread and wait for it to do its thing
            let is_run = run.is_some();
            let system = System::default();
            system
                .spawn_shared(move || async move {
                    // Get the wizard
                    let wizard = self.wizard.take().map(|a| {
                        Box::new(a) as Box<dyn term_api::WizardAbi + Send + Sync + 'static>
                    });

                    // Create the console
                    let fs = wasmer_os::fs::create_root_fs(None);
                    
                    // If a command is passed in then pass it into the console
                    let mut exit_on_return_to_shell = false;
                    let location = if let Some(run) = run.as_ref() {
                        exit_on_return_to_shell = true;
                        format!("ssh://wasmer.sh/?no_welcome&init={}", run)
                    } else {
                        format!("ssh://wasmer.sh/")
                    };
                    
                    let user_agent = "ssh".to_string();
                    let compiled_modules = self.compiled_modules.clone();
                    let mut console = Console::new(
                        location,
                        user_agent,
                        self.compiler,
                        handle,
                        wizard,
                        fs,
                        compiled_modules,
                        self.webc_dir.clone(),
                    );
                    if let Some(engine) = self.engine.clone() {
                        console = console.with_engine(engine.clone());
                    }
                    console.set_exit_on_return_to_shell(exit_on_return_to_shell);
                    console.init().await;
                    if is_run {
                        console.set_raw_mode(true);
                    }
                    self.console.replace(console);

                    // We are ready to receive data
                    Ok((self, session))
                })
                .await
                .unwrap()
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

        // Get the current wizard or fail
        let wizard = match self.wizard.as_mut() {
            Some(a) => a,
            None => {
                return self.finished_auth(Auth::Reject);
            }
        };

        /*
        // Root is always rejected (as this is what bots attack on)
        if user == "root" {
            warn!("root attempt rejected from {}", self.peer_addr_str);
            wizard.fail("root not supported - instead use 'ssh joe@blogs.com@wasmer.sh'\r\n");
        }
        */

        // Set the user if its not set
        if wizard.state.email.is_none() {
            wizard.state.email = Some(user.to_string());
        }

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
            if let Some(console) = self.console.as_mut() {
                console.on_data(&data[..]).await;
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
    fn channel_eof(self, channel: ChannelId, session: Session) -> Self::FutureUnit {
        debug!("channel_eof");

        Box::pin(async move {
            if let Some(console) = self.console.as_ref() {
                console.close_stdin().await;
            }
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

        {
            let mut guard = self.rect.lock().unwrap();
            guard.cols = col_width;
            guard.rows = row_height;
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
