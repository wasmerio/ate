use std::result::Result;
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

    pub fn blocking_open(self) -> Result<WebSocket, std::io::Error> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                self.open().await
            })
        })
    }

    pub async fn open(self) -> Result<WebSocket, std::io::Error> {
        Ok(WebSocket::new(self.url.as_str()).await?)
    }
}
