#![allow(dead_code)]
use std::io::Write;
#[cfg(feature = "tokio")]
use tokio::sync::mpsc;
#[cfg(not(feature = "tokio"))]
use std::sync::mpsc;

use super::*;
use crate::{MAX_MPSC, abi::*};
use crate::backend::ws::*;

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

        let task = call(WAPM_NAME.into(), Connect { url })
            .invoke();
        let recv = task.recv(move |data: Received| {
            let _ = tx_recv.send(data.data);
        });
        
        WebSocket {
            task,
            rx: rx_recv,
            recv
        }
    }
}
