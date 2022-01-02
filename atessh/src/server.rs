use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use term_lib::api::ConsoleRect;
use thrussh::server;
use tokterm::term_lib;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::key::SshServerKey;

pub struct Server {
    pub listen: IpAddr,
    pub port: u16,
    pub server_key: SshServerKey,
    pub connection_timeout: Duration,
    pub auth_rejection_time: Duration,
    pub compiler: term_lib::eval::Compiler,
}

impl Server {
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

        /*
        // Keys will be send via this concurrency structure (and responses by the other)
        let (tx_data, mut rx_data) = tokio::sync::mpsc::channel(term_lib::common::MAX_MPSC);
        let (tx_stdout, mut rx_stdout) = tokio::sync::mpsc::channel(term_lib::common::MAX_MPSC);
        let (tx_stderr, mut rx_stderr) = tokio::sync::mpsc::channel(term_lib::common::MAX_MPSC);
        */

        // Return the handler
        Self::Handler {
            //tx_data,
            //rx_stdout,
            //rx_stderr,
            rect: Arc::new(Mutex::new(ConsoleRect { cols: 80, rows: 25 })),
            compiler: self.compiler,
            console: None,
            peer_addr,
            peer_addr_str,
            user: None,
            client_pubkey: None,
        }
    }
}
