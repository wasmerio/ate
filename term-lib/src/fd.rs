#![allow(dead_code)]
#![allow(unused_imports)]
use bytes::{Buf, BytesMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::ops::Deref;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicU32;
use std::sync::Mutex;
use std::sync::Weak;
use std::{
    pin::Pin,
    sync::Arc,
    task::{self, Context, Poll, Waker},
};
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::Mutex as AsyncMutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::common::*;
use super::err::*;
use super::pipe::*;
use super::poll::*;
use super::reactor::*;
use super::state::*;
use crate::wasmer_vfs::{FileDescriptor, VirtualFile};
use crate::wasmer_wasi::{types as wasi_types, WasiFile, WasiFsError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FdFlag {
    None,
    Stdin(bool),  // bool indicates if the terminal is TTY
    Stdout(bool), // bool indicates if the terminal is TTY
    Stderr(bool), // bool indicates if the terminal is TTY
    Log,
    Tty,
}

impl FdFlag {
    pub fn is_tty(&self) -> bool {
        match self {
            FdFlag::Stdin(tty) => tty.clone(),
            FdFlag::Stdout(tty) => tty.clone(),
            FdFlag::Stderr(tty) => tty.clone(),
            FdFlag::Tty => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FdMsg {
    Data { data: Vec<u8>, flag: FdFlag },
    Flush { tx: mpsc::Sender<()> },
}

impl FdMsg {
    pub fn new(data: Vec<u8>, flag: FdFlag) -> FdMsg {
        FdMsg::Data { data, flag }
    }
    pub fn flush() -> (mpsc::Receiver<()>, FdMsg) {
        let (tx, rx) = mpsc::channel(1);
        let msg = FdMsg::Flush { tx };
        (rx, msg)
    }
    pub fn len(&self) -> usize {
        match self {
            FdMsg::Data { data, .. } => data.len(),
            FdMsg::Flush { .. } => 0usize,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fd {
    pub(crate) flag: FdFlag,
    pub(crate) forced_exit: Arc<AtomicU32>,
    pub(crate) closed: Arc<AtomicBool>,
    pub(crate) blocking: Arc<AtomicBool>,
    pub(crate) sender: Option<Arc<mpsc::Sender<FdMsg>>>,
    pub(crate) receiver: Option<Arc<AsyncMutex<ReactorPipeReceiver>>>,
}

impl Fd {
    pub fn new(
        tx: Option<mpsc::Sender<FdMsg>>,
        rx: Option<Arc<AsyncMutex<ReactorPipeReceiver>>>,
        flag: FdFlag,
    ) -> Fd {
        Fd {
            flag,
            forced_exit: Arc::new(AtomicU32::new(0)),
            closed: Arc::new(AtomicBool::new(false)),
            blocking: Arc::new(AtomicBool::new(true)),
            sender: tx.map(|a| Arc::new(a)),
            receiver: rx,
        }
    }

    pub fn combine(fd1: &Fd, fd2: &Fd) -> Fd {
        let mut ret = Fd {
            flag: fd1.flag,
            forced_exit: fd1.forced_exit.clone(),
            closed: fd1.closed.clone(),
            blocking: Arc::new(AtomicBool::new(fd1.blocking.load(Ordering::Relaxed))),
            sender: None,
            receiver: None,
        };

        if let Some(a) = fd1.sender.as_ref() {
            ret.sender = Some(a.clone());
        } else if let Some(a) = fd2.sender.as_ref() {
            ret.sender = Some(a.clone());
        }

        if let Some(a) = fd1.receiver.as_ref() {
            ret.receiver = Some(a.clone());
        } else if let Some(a) = fd2.receiver.as_ref() {
            ret.receiver = Some(a.clone());
        }

        ret
    }

    pub fn set_blocking(&self, blocking: bool) {
        self.blocking.store(blocking, Ordering::Relaxed);
    }

    pub fn forced_exit(&self, exit_code: u32) {
        self.forced_exit.store(exit_code, Ordering::Release);
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    pub fn is_tty(&self) -> bool {
        self.flag.is_tty()
    }

    pub fn flag(&self) -> FdFlag {
        self.flag
    }

    pub fn set_flag(&mut self, flag: FdFlag) -> FdFlag {
        self.flag = flag;
        flag
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    fn check_closed(&self) -> io::Result<()> {
        if self.is_closed() {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        Ok(())
    }

    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_vec(buf.to_vec()).await
    }

    pub async fn write_vec(&mut self, buf: Vec<u8>) -> io::Result<usize> {
        self.check_closed()?;
        if let Some(sender) = self.sender.as_mut() {
            let buf_len = buf.len();
            let msg = FdMsg::new(buf, self.flag);
            if let Err(_err) = sender.send(msg).await {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            Ok(buf_len)
        } else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
    }

    pub(crate) async fn write_clear_line(&mut self) {
        let _ = self.write("\r\x1b[0K\r".as_bytes()).await;
        let _ = self.flush_async().await;
    }

    pub fn poll(&mut self) -> PollResult {
        poll_fd(
            self.receiver.as_mut(),
            self.sender.as_ref().map(|a| a.deref()),
        )
    }

    pub async fn flush_async(&mut self) -> io::Result<()> {
        let (mut rx, msg) = FdMsg::flush();
        if let Some(sender) = self.sender.as_mut() {
            if let Err(_err) = sender.send(msg).await {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
        } else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        let _ = rx.recv().await;
        Ok(())
    }

    pub async fn read_async(&mut self) -> io::Result<FdMsg> {
        self.check_closed()?;
        if let Some(receiver) = self.receiver.as_mut() {
            let mut receiver = receiver.lock().await;
            if receiver.buffer.has_remaining() {
                let mut buffer = BytesMut::new();
                std::mem::swap(&mut receiver.buffer, &mut buffer);
                return Ok(FdMsg::new(buffer.to_vec(), receiver.cur_flag));
            }
            if receiver.mode == ReceiverMode::Message(true) {
                receiver.mode = ReceiverMode::Message(false);
                return Ok(FdMsg::new(Vec::new(), receiver.cur_flag));
            }
            let msg = receiver
                .rx
                .recv()
                .await
                .unwrap_or(FdMsg::new(Vec::new(), receiver.cur_flag));
            if let FdMsg::Data { flag, .. } = &msg {
                receiver.cur_flag = flag.clone();
            }
            Ok(msg)
        } else {
            return Ok(FdMsg::new(Vec::new(), self.flag));
        }
    }

    fn blocking_send(&mut self, msg: FdMsg) -> io::Result<usize> {
        if let Some(sender) = self.sender.as_mut() {
            let buf_len = msg.len();

            let mut wait_time = 0u32;
            let mut msg = Some(msg);
            loop {
                // Try and send the data
                match sender.try_send(msg.take().unwrap()) {
                    Ok(_) => {
                        return Ok(buf_len);
                    }
                    Err(TrySendError::Full(returned_msg)) => {
                        msg = Some(returned_msg);
                    }
                    Err(TrySendError::Closed(_)) => {
                        return Ok(0);
                    }
                }

                // If we are none blocking then we are done
                if self.blocking.load(Ordering::Relaxed) == false {
                    return Err(std::io::ErrorKind::WouldBlock.into());
                }

                // Check for a forced exit
                let forced_exit = self.forced_exit.load(Ordering::Acquire);
                if forced_exit != 0 {
                    return Err(std::io::ErrorKind::Interrupted.into());
                }

                // Maybe we are closed - if not then yield and try again
                if self.closed.load(Ordering::Acquire) {
                    return Ok(0usize);
                }
            
                // Increase the wait time
                wait_time += 1;
                let wait_time = u32::min(wait_time / 10, 20u32);
                std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
            }
        } else {
            return Ok(0usize);
        }
    }

    fn blocking_recv<T>(&mut self, receiver: &mut mpsc::Receiver<T>) -> io::Result<Option<T>> {
        let mut wait_time = 0u32;
        loop {
            // Try and receive the data
            match receiver.try_recv() {
                Ok(a) => {
                    return Ok(Some(a));
                }
                Err(TryRecvError::Empty) => {
                }
                Err(TryRecvError::Disconnected) => {
                    return Ok(None);
                }
            }

            // If we are none blocking then we are done
            if self.blocking.load(Ordering::Relaxed) == false {
                return Err(std::io::ErrorKind::WouldBlock.into());
            }

            // Check for a forced exit
            let forced_exit = self.forced_exit.load(Ordering::Acquire);
            if forced_exit != 0 {
                return Err(std::io::ErrorKind::Interrupted.into());
            }

            // Maybe we are closed - if not then yield and try again
            if self.closed.load(Ordering::Acquire) {
                return Ok(None);
            }

            // Increase the wait time
            wait_time += 1;
            let wait_time = u32::min(wait_time / 10, 20u32);
            std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
        }
    }
}

impl Seek for Fd {
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        Ok(0u64)
    }
}
impl Write for Fd {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.blocking_send(FdMsg::new(buf.to_vec(), self.flag))
    }

    fn flush(&mut self) -> io::Result<()> {
        let (tx, mut rx) = mpsc::channel(1);
        self.blocking_send(FdMsg::Flush { tx })?;
        self.blocking_recv(&mut rx)?;
        Ok(())
    }
}

impl Read for Fd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(receiver) = self.receiver.as_mut() {
            loop {
                // Make an attempt to read the data
                if let Ok(mut receiver) = receiver.try_lock() {
                    // If we have any data then lets go!
                    if receiver.buffer.has_remaining() {
                        let max = receiver.buffer.remaining().min(buf.len());
                        buf[0..max].copy_from_slice(&receiver.buffer[..max]);
                        receiver.buffer.advance(max);
                        return Ok(max);
                    }

                    // Otherwise lets get some more data
                    match receiver.rx.try_recv() {
                        Ok(msg) => {
                            if let FdMsg::Data { data, flag } = msg {
                                //error!("on_stdin {}", data.iter().map(|byte| format!("\\u{{{:04X}}}", byte).to_owned()).collect::<Vec<String>>().join(""));
                                receiver.cur_flag = flag;
                                receiver.buffer.extend_from_slice(&data[..]);
                                if receiver.mode == ReceiverMode::Message(false) {
                                    receiver.mode = ReceiverMode::Message(true);
                                }
                            }
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {}
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            return Ok(0usize);
                        }
                    }
                }

                // If we are none blocking then we are done
                if self.blocking.load(Ordering::Relaxed) == false {
                    return Err(std::io::ErrorKind::WouldBlock.into());
                }

                // Check for a forced exit
                let forced_exit = self.forced_exit.load(Ordering::Acquire);
                if forced_exit != 0 {
                    return Err(std::io::ErrorKind::Interrupted.into());
                }

                // Maybe we are closed - if not then yield and try again
                if self.closed.load(Ordering::Acquire) {
                    std::thread::yield_now();
                    return Ok(0usize);
                }
                std::thread::park_timeout(std::time::Duration::from_millis(5));
            }
        } else {
            return Ok(0usize);
        }
    }
}

impl VirtualFile for Fd {
    fn last_accessed(&self) -> u64 {
        0
    }
    fn last_modified(&self) -> u64 {
        0
    }
    fn created_time(&self) -> u64 {
        0
    }
    fn size(&self) -> u64 {
        0
    }
    fn set_len(&mut self, _new_size: wasi_types::__wasi_filesize_t) -> Result<(), WasiFsError> {
        Ok(())
    }

    fn unlink(&mut self) -> Result<(), WasiFsError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WeakFd {
    pub(crate) flag: FdFlag,
    pub(crate) forced_exit: Weak<AtomicU32>,
    pub(crate) closed: Weak<AtomicBool>,
    pub(crate) blocking: Weak<AtomicBool>,
    pub(crate) sender: Option<Weak<mpsc::Sender<FdMsg>>>,
    pub(crate) receiver: Option<Weak<AsyncMutex<ReactorPipeReceiver>>>,
}

impl WeakFd {
    pub fn upgrade(&self) -> Option<Fd> {
        let forced_exit = match self.forced_exit.upgrade() {
            Some(a) => a,
            None => {
                return None;
            }
        };

        let closed = match self.closed.upgrade() {
            Some(a) => a,
            None => {
                return None;
            }
        };

        let blocking = match self.blocking.upgrade() {
            Some(a) => a,
            None => {
                return None;
            }
        };

        let sender = self.sender.iter().filter_map(|a| a.upgrade()).next();

        let receiver = self.receiver.iter().filter_map(|a| a.upgrade()).next();

        Some(Fd {
            flag: self.flag,
            forced_exit,
            closed,
            blocking,
            sender,
            receiver,
        })
    }
}

impl Fd {
    pub fn downgrade(&self) -> WeakFd {
        let forced_exit = Arc::downgrade(&self.forced_exit);
        let closed = Arc::downgrade(&self.closed);
        let blocking = Arc::downgrade(&self.blocking);
        let sender = self.sender.iter().map(|a| Arc::downgrade(&a)).next();
        let receiver = self.receiver.iter().map(|a| Arc::downgrade(&a)).next();

        WeakFd {
            flag: self.flag,
            forced_exit,
            closed,
            blocking,
            sender,
            receiver,
        }
    }
}
