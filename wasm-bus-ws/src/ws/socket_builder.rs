#![allow(dead_code)]
use std::io::Write;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;
use crate::api::SocketState;
use wasm_bus::abi::*;

const MAX_MPSC: usize = std::usize::MAX >> 3;

pub struct SocketBuilder {
    pub(crate) url: url::Url,
}

impl SocketBuilder {
    pub fn new(url: url::Url) -> SocketBuilder {
        SocketBuilder { url }
    }

    pub fn open(self) -> WebSocket {
        let url = self.url.to_string();
        let (tx_recv, rx_recv) = mpsc::channel(MAX_MPSC);
        let (tx_state, rx_state) = watch::channel(SocketState::Opening);

        let task = crate::api::SocketBuilder::connect(
            WAPM_NAME,
            url,
            move |data: SocketState| {
                let _ = tx_state.send(data);
            },
            move |data: Vec<u8>| {
                let _ = tx_recv.blocking_send(data);
            },
        );

        WebSocket {
            task: task,
            rx: rx_recv,
            state: rx_state,
        }
    }
}
