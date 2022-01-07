use ate::prelude::*;
use ate_auth::prelude::*;
use ate_files::prelude::*;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use term_lib::api::ConsoleRect;
use thrussh::server;
use tokio::runtime::Runtime;
use tokio::sync::watch;
use tokterm::term_lib;
use tokterm::term_lib::bin_factory::CachedCompiledModules;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::key::SshServerKey;
use crate::opt::OptsSsh;
use crate::wizard::*;

pub struct Server {
    pub listen: IpAddr,
    pub port: u16,
    pub server_key: SshServerKey,
    pub connection_timeout: Duration,
    pub auth_rejection_time: Duration,
    pub compiler: term_lib::eval::Compiler,
    pub registry: Arc<Registry>,
    pub native_files: Arc<FileAccessor>,
    pub auth: url::Url,
    pub compiled_modules: Arc<CachedCompiledModules>,
    pub exit_rx: watch::Receiver<bool>,
}

impl Server {
    pub async fn new(run: OptsSsh, server_key: SshServerKey, runtime: Arc<Runtime>) -> Self {
        // Create the registry that will be used to validate logins
        let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();

        // Connect to the file system that holds all the binaries that
        // we will present natively to the consumers
        // Note: These are the same files presenting to the web-site version of the terminal
        let native_files_key = ate::prelude::ChainKey::from(run.native_files);
        let native_files = registry.open(&run.db, &native_files_key).await.unwrap();
        let native_files = Arc::new(
            FileAccessor::new(
                native_files.as_arc(),
                None,
                AteSessionType::User(AteSessionUser::default()),
                TransactionScope::Local,
                TransactionScope::Local,
                true,
                false,
            )
            .await,
        );

        // Attempt to read the root from the native file system which will make sure that its
        // all nicely running
        native_files
            .search(&RequestContext { uid: 0, gid: 0 }, "/")
            .await
            .unwrap();

        // Set the system
        let (tx_exit, rx_exit) = watch::channel(false);
        let sys = Arc::new(tokterm::system::SysSystem::new_with_runtime(
            tx_exit, runtime,
        ));
        let sys = crate::system::System::new(sys, native_files.clone());
        term_lib::api::set_system_abi(sys);

        // Success
        Self {
            listen: run.listen,
            port: run.port,
            server_key,
            connection_timeout: Duration::from_secs(600),
            auth_rejection_time: Duration::from_secs(0),
            compiler: run.compiler,
            registry,
            native_files,
            auth: run.auth.clone(),
            compiled_modules: Arc::new(CachedCompiledModules::default()),
            exit_rx: rx_exit,
        }
    }
    pub async fn listen(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = thrussh::server::Config::default();
        config.connection_timeout = Some(self.connection_timeout.clone());
        config.auth_rejection_time = self.auth_rejection_time.clone();
        config.keys.push(self.server_key.clone().into());

        let config = Arc::new(config);

        let addr = format!("[{}]:{}", self.listen, self.port);
        info!("listening on {}", addr);

        thrussh::server::run(config, addr.as_str(), self).await?;
        Ok(())
    }
}

impl server::Server for Server {
    type Handler = super::handler::Handler;

    fn new(&mut self, peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        let peer_addr_str = peer_addr
            .map(|a| a.to_string())
            .unwrap_or_else(|| "[unknown]".to_string());
        info!("new connection from {}", peer_addr_str);

        // Return the handler
        let mut wizard = SshWizard {
            step: SshWizardStep::Init,
            state: SshWizardState::default(),
            registry: self.registry.clone(),
            auth: self.auth.clone(),
        };
        wizard.state.welcome = Some(super::cconst::CConst::SSH_WELCOME.to_string());
        Self::Handler {
            rect: Arc::new(Mutex::new(ConsoleRect { cols: 80, rows: 25 })),
            registry: self.registry.clone(),
            native_files: self.native_files.clone(),
            compiler: self.compiler,
            console: None,
            peer_addr,
            peer_addr_str,
            user: None,
            client_pubkey: None,
            wizard: Some(wizard),
            compiled_modules: self.compiled_modules.clone(),
        }
    }
}
