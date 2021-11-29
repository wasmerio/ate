#![allow(unused_imports)]
#![allow(dead_code)]
use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::Mutex as AsyncMutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::common::*;
use super::environment::*;
use super::err::*;
use super::eval::*;
use super::fd::*;
use super::job::*;
use super::poll::*;
use super::stdio::*;
use crate::api::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReceiverMode {
    Stream,
    Message(bool),
}

#[derive(Debug)]
pub struct ReactorPipeReceiver {
    pub(crate) rx: mpsc::Receiver<Vec<u8>>,
    pub(crate) buffer: BytesMut,
    pub(crate) mode: ReceiverMode,
}

pub fn bidirectional(
    buffer_size_tx: usize,
    buffer_size_rx: usize,
    mode: ReceiverMode,
) -> (Fd, mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
    let (tx_read, rx_read) = mpsc::channel(buffer_size_tx);
    let (tx_write, rx_write) = mpsc::channel(buffer_size_rx);
    let fd = Fd::new(
        Some(tx_write),
        Some(Arc::new(AsyncMutex::new(ReactorPipeReceiver {
            rx: rx_read,
            buffer: BytesMut::new(),
            mode,
        }))),
    );
    (fd, tx_read, rx_write)
}

pub fn bidirectional_with_defaults() -> (Fd, mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
    bidirectional(MAX_MPSC, MAX_MPSC, ReceiverMode::Stream)
}

pub fn pipe_out() -> (Fd, mpsc::Receiver<Vec<u8>>) {
    let (tx, rx) = mpsc::channel(MAX_MPSC);
    let fd = Fd::new(Some(tx), None);
    (fd, rx)
}

pub fn pipe_in(mode: ReceiverMode) -> (Fd, mpsc::Sender<Vec<u8>>) {
    let (tx, rx) = mpsc::channel(MAX_MPSC);
    let rx = ReactorPipeReceiver {
        rx,
        buffer: BytesMut::new(),
        mode,
    };
    let fd = Fd::new(None, Some(Arc::new(AsyncMutex::new(rx))));
    (fd, tx)
}

pub fn pipe(mode: ReceiverMode) -> (Fd, Fd) {
    let system = System::default();
    let (fd_rx, tx2) = pipe_in(mode);
    let (fd_tx, mut rx2) = pipe_out();
    system.fork_local(async move {
        while let Some(data) = rx2.recv().await {
            let _ = tx2.send(data).await;
        }
    });
    (fd_tx, fd_rx)
}
