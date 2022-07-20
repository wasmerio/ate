use ate::mesh::Registry;
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

use crate::native_files::NativeFileInterface;
use crate::wizard::SshWizard;

use super::console_handle::*;
use super::error::*;

pub struct Handler {
    pub registry: Arc<Registry>,
    pub native_files: NativeFileInterface,
    pub peer_addr: Option<std::net::SocketAddr>,
    pub peer_addr_str: String,
    pub user: Option<String>,
    pub client_pubkey: Option<thrussh_keys::key::PublicKey>,
    pub console: Option<Console>,
    pub compiler: wasmer_os::eval::Compiler,
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub wizard: Option<SshWizard>,
    pub compiled_modules: Arc<CachedCompiledModules>,
    pub stdio_lock: Arc<Mutex<()>>,
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

        // Root is always rejected (as this is what bots attack on)
        if user == "root" {
            warn!("root attempt rejected from {}", self.peer_addr_str);
            wizard.fail("root not supported - instead use 'ssh joe@blogs.com@wasmer.sh'\r\n");
        }

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
        let data = String::from_utf8(data.to_vec()).map_err(|_| {
            let err: SshServerError = SshServerErrorKind::BadData.into();
            err
        });
        Box::pin(async move {
            let data = data?;
            if let Some(console) = self.console.as_mut() {
                console.on_data(data).await;
            }
            Ok((self, session))
        })
    }

    #[allow(unused_variables)]
    fn shell_request(mut self, channel: ChannelId, session: Session) -> Self::FutureUnit {
        debug!("shell_request");

        let native_files = self.native_files.clone();
        Box::pin(async move {
            // Create the handle
            let handle = Arc::new(ConsoleHandle {
                rect: self.rect.clone(),
                channel: channel.clone(),
                handle: session.handle(),
                stdio_lock: self.stdio_lock.clone(),
                enable_stderr: false,
            });

            // Spawn a dedicated thread and wait for it to do its thing
            let system = System::default();
            system
                .spawn_shared(move || async move {
                    // Get the wizard
                    let wizard = self.wizard.take().map(|a| {
                        Box::new(a) as Box<dyn term_api::WizardAbi + Send + Sync + 'static>
                    });

                    // Create the console
                    let fs = wasmer_os::fs::create_root_fs(None);
                    let location = "ssh://wasmer.sh/?no_welcome".to_string();
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
                    );
                    console.init().await;
                    self.console.replace(console);

                    // We are ready to receive data
                    Ok((self, session))
                })
                .await
                .unwrap()
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
