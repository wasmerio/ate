use std::result::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;

pub struct SocketBuilder {
    pub(crate) url: url::Url,
}

impl SocketBuilder {
    pub fn new(url: url::Url) -> SocketBuilder {
        SocketBuilder { url }
    }

    pub fn new_str(url: &str) -> Result<SocketBuilder, url::ParseError> {
        let url = url::Url::parse(url)?;
        Ok(SocketBuilder { url })
    }

    pub fn blocking_open(self) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, std::io::Error> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                self.open().await
            })
        })
    }

    pub async fn open(self) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, std::io::Error> {
        Ok(super::web_socket::connect(self.url.as_str()).await?)
    }
}
