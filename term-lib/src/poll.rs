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
}

pub fn poll_fd(
    rx: Option<&mut Arc<AsyncMutex<ReactorPipeReceiver>>>,
    tx: Option<&mpsc::Sender<FdMsg>>,
) -> PollResult {
    let mut has_fd = false;
    let can_write = if let Some(fd) = tx {
        has_fd = true;
        if let Ok(_permit) = fd.try_reserve() {
            true
        } else {
            false
        }
    } else {
        false
    };
    let can_read = if let Some(fd) = rx {
        has_fd = true;
        let mut fd = fd.blocking_lock();
        if fd.buffer.is_empty() == false {
            true
        } else if let Ok(msg) = fd.rx.try_recv() {
            match msg {
                FdMsg::Data { data, flag } => {
                    fd.cur_flag = flag;
                    fd.buffer.extend_from_slice(&data[..]);
                }
                FdMsg::Flush { tx } => {
                    let _ = tx.try_send(());
                }
            }
            true
        } else {
            false
        }
    } else {
        false
    };
    let ret = PollResult {
        can_read,
        can_write,
        is_closed: has_fd == false,
    };
    ret
}
