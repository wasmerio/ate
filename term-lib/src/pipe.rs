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
    // Partial reads are allowed
    Stream,
    // Reads will return entire messages (flag indicates if zero terminated)
    Message(bool),
}

#[derive(Debug)]
pub struct ReactorPipeReceiver {
    pub(crate) rx: mpsc::Receiver<FdMsg>,
    pub(crate) buffer: BytesMut,
    pub(crate) mode: ReceiverMode,
    pub(crate) cur_flag: FdFlag,
}

pub fn bidirectional(
    buffer_size_tx: usize,
    buffer_size_rx: usize,
    mode: ReceiverMode,
    flag: FdFlag,
) -> (Fd, mpsc::Sender<FdMsg>, mpsc::Receiver<FdMsg>) {
    let (tx_read, rx_read) = mpsc::channel(buffer_size_tx);
    let (tx_write, rx_write) = mpsc::channel(buffer_size_rx);
    let fd = Fd::new(
        Some(tx_write),
        Some(rx_read),
        mode,
        flag,
    );
    (fd, tx_read, rx_write)
}

pub fn bidirectional_with_defaults(
    flag: FdFlag,
) -> (Fd, mpsc::Sender<FdMsg>, mpsc::Receiver<FdMsg>) {
    bidirectional(MAX_MPSC, MAX_MPSC, ReceiverMode::Stream, flag)
}

pub fn pipe_out(flag: FdFlag) -> (Fd, mpsc::Receiver<FdMsg>) {
    let (tx, rx) = mpsc::channel(MAX_MPSC);
    let fd = Fd::new(Some(tx), None, ReceiverMode::Stream, flag);
    (fd, rx)
}

pub fn pipe_in(mode: ReceiverMode, flag: FdFlag) -> (Fd, mpsc::Sender<FdMsg>) {
    let (tx, rx) = mpsc::channel(MAX_MPSC);
    let fd = Fd::new(None, Some(rx), mode, flag);
    (fd, tx)
}

pub fn pipe(mode: ReceiverMode, flag: FdFlag) -> (Fd, Fd) {
    let system = System::default();
    let (fd_rx, tx2) = pipe_in(mode, flag);
    let (fd_tx, mut rx2) = pipe_out(flag);
    system.fork_shared(move || async move {
        while let Some(data) = rx2.recv().await {
            let _ = tx2.send(data).await;
        }
    });
    (fd_tx, fd_rx)
}
