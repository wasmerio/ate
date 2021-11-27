use std::io;
use std::io::Read;
use std::io::Write;
#[cfg(not(feature = "tokio"))]
use std::sync::mpsc;
#[cfg(feature = "tokio")]
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::abi::*;
use crate::backend::ws::*;

#[derive(Debug)]
pub struct WebSocket {
    pub(super) task: Call,
    pub(super) rx: mpsc::Receiver<Vec<u8>>,
    #[allow(dead_code)]
    pub(super) recv: Recv,
}

impl WebSocket {
    pub fn split(self) -> (SendHalf, RecvHalf) {
        (SendHalf { task: self.task }, RecvHalf { rx: self.rx, recv: self.recv })
    }
}

#[derive(Debug, Clone)]
pub struct SendHalf {
    task: Call,
}

impl SendHalf {
    pub async fn send(&self, data: Vec<u8>) -> io::Result<usize> {
        self.task
            .call(Send { data })
            .invoke()
            .join()
            .await
            .map_err(|err| err.into_io_error())
            .map(|ret| {
                match ret {
                    SendResult::Success(a) => Ok(a),
                    SendResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err))
                }
            })
    }

    pub fn blocking_send(&self, data: Vec<u8>) -> io::Result<usize> {
        self.task
            .call(Send { data })
            .invoke()
            .join()
            .wait()
            .map_err(|err| err.into_io_error())
            .map_err(|e| e.into())
            .map(|ret| {
                match ret {
                    SendResult::Success(a) => Ok(a),
                    SendResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err))
                }
            })
    }
}

#[derive(Debug)]
pub struct RecvHalf {
    rx: mpsc::Receiver<Vec<u8>>,
    #[allow(dead_code)]
    recv: Recv,
}

#[cfg(feature = "tokio")]
impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        self.rx.blocking_recv()
    }
}

#[cfg(not(feature = "tokio"))]
impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().ok()
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().ok()
    }
}
