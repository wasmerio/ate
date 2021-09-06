use std::net::SocketAddr;

use ate::prelude::*;

#[derive(Debug, Clone)]
pub struct ServerListen
{
    pub addr: SocketAddr,
    pub tls: bool,
    pub http2: bool,
}

#[derive(Debug, Default)]
pub struct ServerConf
{
    pub cfg_ate: ConfAte,
    pub listen: Vec<ServerListen>,
}