#![allow(unused_imports)]
#![allow(dead_code)]
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::err::*;
use super::fd::*;
use super::reactor::*;
use super::pipe::*;

#[derive(Debug)]
pub struct PollResult {
    pub can_read: bool,
    pub can_write: bool,
    pub is_closed: bool,
}

pub fn poll_fd(
    rx: Option<&mut Arc<Mutex<ReactorPipeReceiver>>>,
    tx: Option<&mut mpsc::Sender<Vec<u8>>>,
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
        let mut fd = fd.lock().unwrap();
        if let Ok(data) = fd.rx.try_recv() {
            fd.buffer.extend_from_slice(&data[..]);
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
