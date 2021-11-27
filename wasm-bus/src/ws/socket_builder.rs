#![allow(dead_code)]
use std::io::Write;
#[cfg(not(feature = "tokio"))]
use std::sync::mpsc;
#[cfg(feature = "tokio")]
use tokio::sync::mpsc;
#[cfg(not(feature = "tokio"))]
use std::sync::watch;
#[cfg(feature = "tokio")]
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;
use crate::backend::ws::*;
use crate::{abi::*, MAX_MPSC};

pub struct SocketBuilder {
    pub(crate) url: url::Url,
}

impl SocketBuilder {
    pub fn new(url: url::Url) -> SocketBuilder {
        SocketBuilder { url }
    }

    pub fn open(self) -> WebSocket {
        let url = self.url.to_string();

        #[cfg(feature = "tokio")]
        let (tx_recv, rx_recv) = mpsc::channel(MAX_MPSC);
        #[cfg(not(feature = "tokio"))]
        let (tx_recv, rx_recv) = mpsc::channel();

        #[cfg(feature = "tokio")]
        let (tx_state, rx_state) = watch::channel(SocketState::Opening);
        #[cfg(not(feature = "tokio"))]
        let (tx_state, rx_state) = watch::channel();

        let mut task = call(WAPM_NAME.into(), Connect { url });
        task.callback(move |data: SocketState| {
            let _ = tx_state.send(data);
        });
        task.callback(move |data: Received| {
            let _ = tx_recv.send(data);
        });

        WebSocket {
            task: task.invoke(),
            rx: rx_recv,
            state: rx_state,
        }
    }
}
