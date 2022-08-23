use std::net::SocketAddr;
use std::time::Duration;

#[cfg(feature = "ate")]
use ate::prelude::*;

#[derive(Debug, Clone)]
pub struct ServerListen {
    pub addr: SocketAddr,
    pub tls: bool,
}

#[derive(Debug)]
pub struct ServerConf {
    #[cfg(feature = "ate")]
    pub cfg_ate: ConfAte,
    pub ttl: Duration,
    pub listen: Vec<ServerListen>,
}

impl Default for ServerConf {
    fn default() -> Self {
        ServerConf {
            #[cfg(feature = "ate")]
            cfg_ate: ConfAte::default(),
            ttl: Duration::from_secs(60),
            listen: Vec::new(),
        }
    }
}
