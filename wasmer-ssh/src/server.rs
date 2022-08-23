use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use wasmer_os::api::ConsoleRect;
use thrussh::server;
use wasmer_term::wasmer_os;
use wasmer_os::bin_factory::CachedCompiledModules;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::key::SshServerKey;
use crate::native_files::NativeFileType;
use crate::opt::*;
use crate::wizard::*;

pub struct Server {
    pub listen: IpAddr,
    pub port: u16,
    pub server_key: SshServerKey,
    pub connection_timeout: Duration,
    pub auth_rejection_time: Duration,
    pub engine: Option<wasmer_os::wasmer::Engine>,
    pub compiler: wasmer_os::eval::Compiler,
    pub native_files: NativeFileType,
    pub compiled_modules: Arc<CachedCompiledModules>,
    pub webc_dir: Option<String>,
    pub stdio_lock: Arc<Mutex<()>>,
}

impl Server {
    pub async fn new(host: OptsHost, server_key: SshServerKey, compiled_modules: Arc<CachedCompiledModules>, webc_dir: Option<String>, native_files: NativeFileType) -> Self {
        // Success
        let engine = host.compiler.new_engine();
        Self {
            native_files,
            listen: host.listen,
            port: host.port,
            server_key,
            connection_timeout: Duration::from_secs(600),
            auth_rejection_time: Duration::from_secs(0),
            compiler: host.compiler,
            engine,
            compiled_modules,
            webc_dir,
            stdio_lock: Arc::new(Mutex::new(())),
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

    fn new(&mut self, peer_addr: Option<std::net::SocketAddr>) -> super::handler::Handler {
        let peer_addr_str = peer_addr
            .map(|a| a.to_string())
            .unwrap_or_else(|| "[unknown]".to_string());
        info!("new connection from {}", peer_addr_str);

        // Return the handler
        let mut wizard = SshWizard {
            step: SshWizardStep::Init,
            state: SshWizardState::default(),
        };
        wizard.state.welcome = Some(super::cconst::CConst::SSH_WELCOME.to_string());
        super::handler::Handler {
            rect: Arc::new(Mutex::new(ConsoleRect { cols: 80, rows: 25 })),
            engine: self.engine.clone(),
            compiler: self.compiler,
            console: None,
            peer_addr,
            peer_addr_str,
            user: None,
            client_pubkey: None,
            wizard: Some(wizard),
            compiled_modules: self.compiled_modules.clone(),
            webc_dir: self.webc_dir.clone(),
            stdio_lock: self.stdio_lock.clone(),
        }
    }
}
