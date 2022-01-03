use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use term_lib::api::ConsoleRect;
use term_lib::api::System;
use term_lib::console::Console;
use thrussh::server;
use thrussh::server::Auth;
use thrussh::server::Session;
use thrussh::ChannelId;
use thrussh_keys::key::ed25519;
use thrussh_keys::key::PublicKey;
use tokterm::term_lib;
use tokterm::term_lib::api::SystemAbiExt;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::console_handle::*;
use super::error::*;

pub struct Handler {
    pub peer_addr: Option<std::net::SocketAddr>,
    pub peer_addr_str: String,
    pub user: Option<String>,
    pub client_pubkey: Option<thrussh_keys::key::PublicKey>,
    pub console: Option<Console>,
    pub compiler: term_lib::eval::Compiler,
    pub rect: Arc<Mutex<ConsoleRect>>,
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

    fn auth_publickey(
        mut self,
        user: &str,
        public_key: &thrussh_keys::key::PublicKey,
    ) -> Self::FutureAuth {
        info!("authenticated with pubkey");
        self.user = Some(user.to_string());
        self.client_pubkey = Some(clone_public_key(public_key));
        self.finished_auth(Auth::Accept)
    }

    fn auth_none(mut self, user: &str) -> Self::FutureAuth {
        info!("authenticated with none");
        self.user = Some(user.to_string());
        self.finished_auth(Auth::Accept)
    }

    fn auth_keyboard_interactive(
        mut self,
        user: &str,
        _submethods: &str,
        _response: Option<server::Response>,
    ) -> Self::FutureAuth {
        info!("authenticated with keyboard interactive");
        self.user = Some(user.to_string());
        self.finished_auth(Auth::Accept)
    }

    fn data(mut self, channel: ChannelId, data: &[u8], session: Session) -> Self::FutureUnit {
        info!(
            "data on channel {:?}: {:?}",
            channel,
            std::str::from_utf8(data)
        );
        let data = String::from_utf8(data.to_vec()).map_err(|_| {
            let err: SshServerError = SshServerErrorKind::BadData.into();
            err
        });
        Box::pin(async move {
            let data = data?;
            session.flush();
            if let Some(console) = self.console.as_mut() {
                console.on_data(data).await;
            }
            session.flush();
            Ok((self, session))
        })
    }

    #[allow(unused_variables)]
    fn shell_request(mut self, channel: ChannelId, mut session: Session) -> Self::FutureUnit {
        info!("shell_request");

        Box::pin(async move {
            // Create the handle
            let handle = Arc::new(ConsoleHandle {
                rect: self.rect.clone(),
                channel: channel.clone(),
                handle: session.handle(),
            });

            // Spawn a dedicated thread and wait for it to do its thing
            let system = System::default();
            system
                .spawn_dedicated(move || async move {
                    // Create the console
                    let location = "wss://localhost/".to_string();
                    let user_agent = "noagent".to_string();
                    let mut console = Console::new(location, user_agent, self.compiler, handle);
                    console.init().await;
                    self.console.replace(console);

                    // We are ready to receive data
                    Ok((self, session))
                })
                .join()
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
        info!("pty_request");
        self.finished(session)
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        info!("connection closed ({})", self.peer_addr_str);
    }
}

fn clone_public_key(key: &PublicKey) -> PublicKey {
    match key {
        PublicKey::Ed25519(a) => PublicKey::Ed25519(ed25519::PublicKey { key: a.key.clone() }),
    }
}
