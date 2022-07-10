#![allow(dead_code)]
use std::io::Write;
use std::result::Result;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;
use crate::api;
use crate::model::SocketState;
use wasmer_bus::abi::*;

const MAX_MPSC: usize = std::usize::MAX >> 3;

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

    pub fn blocking_open(self) -> Result<WebSocket, std::io::Error> {
        wasmer_bus::task::block_on(self.open())
    }

    pub async fn open(self) -> Result<WebSocket, std::io::Error> {
        use api::SocketBuilder;

        let url = self.url.to_string();
        let (tx_recv, rx_recv) = mpsc::channel(MAX_MPSC);
        let (tx_state, rx_state) = watch::channel(SocketState::Opening);

        let client = api::SocketBuilderClient::new(WAPM_NAME)
            .connect(
                url,
                Box::new(move |data: SocketState| {
                    let _ = tx_state.send(data);
                }),
                Box::new(move |data: Vec<u8>| {
                    let _ = tx_recv.blocking_send(data);
                }),
            )
            .await
            .map_err(|err| err.into_io_error())?;

        Ok(WebSocket {
            client,
            rx: rx_recv,
            state: rx_state,
        })
    }
}
