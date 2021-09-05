use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use url::Url;

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
    pub fn add_listener(mut self, ip: IpAddr, port: u16, tls: bool, http2: bool) -> Self {
        self.conf.listen.push(ServerListen {
            addr: SocketAddr::new(ip, port),
            tls,
            http2
        });
        self
    }

    pub async fn build(self) -> Arc<Server> {
        Server::new(self).await
    }
}