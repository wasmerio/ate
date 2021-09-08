use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use url::Url;
use std::time::Duration;

use ate::conf::ConfAte;

use super::conf::*;
use super::server::*;

pub struct ServerBuilder
{
    pub(crate) remote: Url,
    pub(crate) conf: ServerConf,
    pub(crate) callback: Option<Arc<dyn ServerCallback>>,
}

impl ServerBuilder
{
    pub fn new(remote: Url) -> ServerBuilder
    {
        ServerBuilder {
            remote,
            conf: ServerConf::default(),
            callback: None,
        }
    }

    pub fn with_conf(mut self, cfg: &ConfAte) -> Self {
        self.conf.cfg_ate = cfg.clone();
        self
    }

    pub fn with_callback(mut self, callback: impl ServerCallback + 'static) -> Self {
        let callback = Arc::new(callback);
        self.callback = Some(callback);
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