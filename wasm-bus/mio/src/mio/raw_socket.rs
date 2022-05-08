use std::io;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::comms::*;

pub struct AsyncRawSocket {
    socket: Mutex<Socket>,
}

impl AsyncRawSocket {
    pub(crate) fn new(socket: Socket) -> Self {
        Self {
            socket: Mutex::new(socket)
        }
    }

    pub async fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        let socket = self.socket.lock().await;
        socket
            .send(buf)
            .await
    }

    pub async fn recv(&self) -> io::Result<Vec<u8>> {
        let mut socket = self.socket.lock().await;
        socket
            .recv()
            .await
    }

    pub fn try_recv(&self) -> io::Result<Option<Vec<u8>>> {
        if let Ok(mut socket) = self.socket.try_lock() {
            socket
                .try_recv()
        } else {
            Ok(None)
        }        
    }

    pub async fn set_promiscuous(&self, promiscuous: bool) -> io::Result<bool> {
        let mut socket = self.socket.lock().await;
        socket.set_promiscuous(promiscuous).await
    }

    pub fn blocking(self) -> super::RawSocket {
        super::RawSocket::new(self)
    }
}
