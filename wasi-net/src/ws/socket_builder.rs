#![allow(dead_code)]
use std::io::Write;
use tokio::sync::mpsc;

use super::*;
use crate::abi::*;
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

        let (tx_recv, rx_recv) = mpsc::channel(crate::MAX_MPSC);
        let task = call(WAPM_NAME, Connect { url })
            .with_callback(move |data: Received| {
                let tx_recv = tx_recv.clone();
                async move {
                    let _ = tx_recv.send(data.data).await;
                    CallbackLifetime::KeepGoing
                }
            })
            .invoke();
        
        WebSocket {
            task,
            rx: rx_recv
        }
    }
}
