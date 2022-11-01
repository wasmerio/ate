use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use thrussh::server;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use wasmer_wasi::bin_factory::CachedCompiledModules;

use crate::key::SshServerKey;
use crate::opt::*;

pub struct Server {
    pub listen: IpAddr,
    pub port: u16,
    pub server_key: SshServerKey,
    pub connection_timeout: Duration,
    pub auth_rejection_time: Duration,
    pub compiled_modules: Arc<CachedCompiledModules>,
}

impl Server {
    pub async fn new(host: OptsHost, server_key: SshServerKey, compiled_modules: Arc<CachedCompiledModules>) -> Self {
        // Success
        Self {
            listen: host.listen,
            port: host.port,
            server_key,
            connection_timeout: Duration::from_secs(600),
            auth_rejection_time: Duration::from_secs(0),
            compiled_modules,
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
        super::handler::Handler {
            tty: None,
            console: None,
            peer_addr,
            peer_addr_str,
            user: None,
            client_pubkey: None,
            compiled_modules: self.compiled_modules.clone(),
        }
    }
}
