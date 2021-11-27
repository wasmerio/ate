use std::io;
use std::io::Read;
use std::io::Write;
#[cfg(not(feature = "tokio"))]
use std::sync::mpsc;
#[cfg(not(feature = "tokio"))]
use std::sync::watch;
#[cfg(feature = "tokio")]
use tokio::sync::mpsc;
#[cfg(feature = "tokio")]
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::abi::*;
use crate::backend::ws::*;

#[derive(Debug)]
pub struct WebSocket {
    pub(super) task: Call,
    pub(super) rx: mpsc::Receiver<Received>,
    pub(super) state: watch::Receiver<SocketState>,
}

impl WebSocket {
    pub fn split(self) -> (SendHalf, RecvHalf) {
        (
            SendHalf {
                task: self.task,
                state: self.state,
            },
            RecvHalf { rx: self.rx },
        )
    }
}

#[derive(Debug, Clone)]
pub struct SendHalf {
    task: Call,
    state: watch::Receiver<SocketState>,
}

impl SendHalf {
    pub async fn wait_till_opened(&self) -> SocketState {
        let mut state = self.state.clone();
        while *state.borrow() == SocketState::Opening {
            if let Err(_) = state.changed().await {
                return SocketState::Closed;
            }
        }
        let ret = (*state.borrow()).clone();
        ret
    }

    pub async fn send(&self, data: Vec<u8>) -> io::Result<usize> {
        if self.wait_till_opened().await != SocketState::Opened {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "connection is not open",
            ));
        }
        self.task
            .call(Send { data })
            .invoke()
            .join()
            .await
            .map_err(|err| err.into_io_error())
            .map(|ret| match ret {
                SendResult::Success(a) => Ok(a),
                SendResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
            })?
    }

    pub fn blocking_send(&self, data: Vec<u8>) -> io::Result<usize> {
        if *self.state.borrow() != SocketState::Opened {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "connection is not open",
            ));
        }
        self.task
            .call(Send { data })
            .invoke()
            .join()
            .wait()
            .map_err(|err| err.into_io_error())
            .map(|ret| match ret {
                SendResult::Success(a) => Ok(a),
                SendResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
            })?
    }
}

#[derive(Debug)]
pub struct RecvHalf {
    rx: mpsc::Receiver<Received>,
}

#[cfg(feature = "tokio")]
impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await.map(|a| a.data)
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        self.rx.blocking_recv().map(|a| a.data)
    }
}

#[cfg(not(feature = "tokio"))]
impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().ok().map(|a| a.data)
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().ok().map(|a| a.data)
    }
}
