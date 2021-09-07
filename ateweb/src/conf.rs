use std::net::SocketAddr;
use std::time::Duration;

use ate::prelude::*;

#[derive(Debug, Clone)]
pub struct ServerListen
{
    pub addr: SocketAddr,
    pub tls: bool,
}

#[derive(Debug)]
pub struct ServerConf
{
    pub cfg_ate: ConfAte,
    pub ttl: Duration,
    pub listen: Vec<ServerListen>,
}

impl Default
for ServerConf
{
    fn default() -> Self
    {
        ServerConf
        {
            cfg_ate: ConfAte::default(),
            ttl: Duration::from_secs(60),
            listen: Vec::new(),
        }
    }
}