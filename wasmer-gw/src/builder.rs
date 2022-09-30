use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports)]
use url::Url;

#[cfg(feature = "ate")]
use ate::prelude::*;

use super::conf::*;
use super::server::*;

pub struct ServerBuilder {
    #[cfg(feature = "dfs")]
    pub(crate) remote: Url,
    #[cfg(feature = "wasmer-auth")]
    pub(crate) auth_url: Url,
    pub(crate) conf: ServerConf,
    #[cfg(feature = "ate")]
    pub(crate) web_master_key: Option<EncryptKey>,
    #[cfg(feature = "ate")]
    pub(crate) session_cert_store: Option<AteSessionGroup>,
    pub(crate) callback: Option<Arc<dyn ServerCallback>>,
    pub(crate) www_path: Option<String>,
}

impl ServerBuilder {
    pub fn new(
        #[cfg(feature = "dfs")]
        remote: Url,
        #[cfg(feature = "dfs")]
        auth_url: Url
    ) -> ServerBuilder {
        ServerBuilder {
            www_path: None,
            #[cfg(feature = "dfs")]
            remote,
            #[cfg(feature = "dfs")]
            auth_url,
            conf: ServerConf::default(),
            #[cfg(feature = "ate")]
            web_master_key: None,
            #[cfg(feature = "ate")]
            session_cert_store: None,
            callback: None,
        }
    }

    pub fn with_www_path(mut self, path: String) -> Self {
        self.www_path = Some(path);
        self
    }

    #[cfg(feature = "ate")]
    pub fn with_web_master_key(mut self, key: EncryptKey) -> Self {
        self.web_master_key = Some(key);
        self
    }

    #[cfg(feature = "ate")]
    pub fn with_conf(mut self, cfg: &ConfAte) -> Self {
        self.conf.cfg_ate = cfg.clone();
        self
    }

    #[cfg(feature = "ate")]
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

    pub fn add_listener(
        mut self,
        ip: IpAddr,
        port: u16,
        tls: bool
    ) -> Self {
        self.conf.listen.push(ServerListen {
            addr: SocketAddr::new(ip, port),
            tls,
        });
        self
    }

    pub async fn build(self) -> Result<Arc<Server>, Box<dyn Error>> {
        Server::new(self).await
    }
}
