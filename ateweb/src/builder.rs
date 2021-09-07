use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use url::Url;
use std::time::Duration;

use ate::conf::ConfAte;

use super::conf::*;
use super::server::*;

#[derive(Debug)]
pub struct ServerBuilder
{
    pub(crate) remote: Url,
    pub(crate) conf: ServerConf,
}

impl ServerBuilder
{
    pub fn new(remote: Url) -> ServerBuilder
    {
        ServerBuilder {
            remote,
            conf: ServerConf::default(),
        }
    }

    pub fn with_conf(mut self, cfg: &ConfAte) -> Self {
        self.conf.cfg_ate = cfg.clone();
        self
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.conf.ttl = ttl;
        self
    }

    pub fn add_listener(mut self, ip: IpAddr, port: u16, tls: bool) -> Self {
        self.conf.listen.push(ServerListen {
            addr: SocketAddr::new(ip, port),
            tls,
        });
        self
    }

    pub async fn build(self) -> Arc<Server> {
        Server::new(self).await
    }
}