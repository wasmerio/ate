use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use url::Url;
use std::time::Duration;

use ate::prelude::*;

use super::conf::*;
use super::server::*;

pub struct ServerBuilder
{
    pub(crate) remote: Url,
    pub(crate) auth_url: Url,
    pub(crate) conf: ServerConf,
    pub(crate) web_key: EncryptKey,
    pub(crate) session_cert_store: Option<AteSessionGroup>,
    pub(crate) callback: Option<Arc<dyn ServerCallback>>,
}

impl ServerBuilder
{
    pub fn new(remote: Url, auth_url: Url, web_key: EncryptKey) -> ServerBuilder
    {
        ServerBuilder {
            remote,
            auth_url,
            conf: ServerConf::default(),
            web_key,
            session_cert_store: None,
            callback: None,
        }
    }

    pub fn with_conf(mut self, cfg: &ConfAte) -> Self {
        self.conf.cfg_ate = cfg.clone();
        self
    }

    pub fn with_cert_store_session(mut self, session_cert_store: AteSessionGroup) -> Self {
        self.session_cert_store = Some(session_cert_store);
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

    pub async fn build(self) -> Result<Arc<Server>, AteError> {
        Server::new(self).await
    }
}