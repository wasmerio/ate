#![allow(unused_imports)]
#![allow(dead_code)]
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::err::*;
use super::fd::*;
use super::pipe::*;
use super::reactor::*;

#[derive(Debug)]
pub struct PollResult {
    pub can_read: bool,
    pub can_write: bool,
    pub is_closed: bool,
    pub bytes_available_read: usize,
}

pub fn poll_fd(
    rx: Option<&Arc<AsyncMutex<ReactorPipeReceiver>>>,
    tx: Option<&mpsc::Sender<FdMsg>>,
) -> PollResult {
    let mut bytes_available_read = 0usize;
    
    let can_write = if let Some(fd) = tx {
        match fd.try_reserve() {
            Ok(_permit) => {
                true
            }
            Err(mpsc::error::TrySendError::Full(())) => {
                false
            }
            Err(mpsc::error::TrySendError::Closed(())) => {
                return PollResult {
                    can_read: false,
                    can_write: false,
                    is_closed: true,
                    bytes_available_read: 0
                };
            }
        }
    } else {
        false
    };
    let can_read = if let Some(fd) = rx {
        match fd.try_lock() {
            Ok(mut fd) => {
                if fd.buffer.is_empty() == false {
                    bytes_available_read += fd.buffer.len();
                    true
                } else {
                    match fd.rx.try_recv() {
                        Ok(msg) => {
                            match msg {
                                FdMsg::Data { data, flag } => {
                                    fd.cur_flag = flag;
                                    fd.buffer.extend_from_slice(&data[..]);
                                    if fd.mode == ReceiverMode::Message(false) {
                                        fd.mode = ReceiverMode::Message(true);
                                    }
                                    bytes_available_read += fd.buffer.len();
                                    true
                                }
                                FdMsg::Flush { tx } => {
                                    let _ = tx.try_send(());
                                    false
                                }
                            }
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {
                            false
                        }
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            return PollResult {
                                can_read: false,
                                can_write: false,
                                is_closed: true,
                                bytes_available_read: 0
                            };
                        }
                    }
                }
            },
            Err(_) => {
                false
            }
        }
    } else {
        false
    };
    let ret = PollResult {
        can_read,
        can_write,
        is_closed: rx.is_some() || tx.is_some(),
        bytes_available_read,
    };
    ret
}
