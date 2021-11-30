#![allow(dead_code)]
#![allow(unused_imports)]
use bytes::{Buf, BytesMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::ops::Deref;
use std::sync::atomic::AtomicI32;
use std::sync::Mutex;
use std::sync::Weak;
use std::sync::atomic::AtomicU32;
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

#[derive(Debug, Clone)]
pub struct Fd {
    pub(crate) forced_exit: Arc<AtomicU32>,
    pub(crate) closed: Arc<AtomicBool>,
    pub(crate) blocking: Arc<AtomicBool>,
    pub(crate) sender: Option<Arc<mpsc::Sender<Vec<u8>>>>,
    pub(crate) receiver: Option<Arc<AsyncMutex<ReactorPipeReceiver>>>,
}

impl Fd {
    pub fn new(
        tx: Option<mpsc::Sender<Vec<u8>>>,
        rx: Option<Arc<AsyncMutex<ReactorPipeReceiver>>>,
    ) -> Fd {
        Fd {
            forced_exit: Arc::new(AtomicU32::new(0)),
            closed: Arc::new(AtomicBool::new(false)),
            blocking: Arc::new(AtomicBool::new(true)),
            sender: tx.map(|a| Arc::new(a)),
            receiver: rx,
        }
    }

    pub fn combine(fd1: &Fd, fd2: &Fd) -> Fd {
        let mut ret = Fd {
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

    pub fn forced_exit(&self, exit_code: i32) {
        self.forced_exit.store(exit_code as u32, Ordering::Release);
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
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
        self.check_closed()?;
        if let Some(sender) = self.sender.as_mut() {
            let buf_len = buf.len();
            let buf = buf.to_vec();
            if let Err(_err) = sender.send(buf).await {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            Ok(buf_len)
        } else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
    }

    pub async fn write_vec(&mut self, buf: Vec<u8>) -> io::Result<usize> {
        self.check_closed()?;
        if let Some(sender) = self.sender.as_mut() {
            let buf_len = buf.len();
            if let Err(_err) = sender.send(buf).await {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            Ok(buf_len)
        } else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
    }

    pub(crate) async fn write_clear_line(&mut self) {
        let _ = self.write("\r\x1b[0K\r".as_bytes()).await;
    }

    pub fn poll(&mut self) -> PollResult {
        poll_fd(
            self.receiver.as_mut(),
            self.sender.as_ref().map(|a| a.deref()),
        )
    }

    pub async fn read_async(&mut self) -> io::Result<Vec<u8>> {
        self.check_closed()?;
        if let Some(receiver) = self.receiver.as_mut() {
            let mut receiver = receiver.lock().await;
            if receiver.buffer.has_remaining() {
                let mut buffer = BytesMut::new();
                std::mem::swap(&mut receiver.buffer, &mut buffer);
                return Ok(buffer.to_vec());
            }
            if receiver.mode == ReceiverMode::Message(true) {
                receiver.mode = ReceiverMode::Message(false);
                return Ok(Vec::new());
            }
            Ok(receiver.rx.recv().await.unwrap_or(Vec::new()))
        } else {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
    }
}

impl Seek for Fd {
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(io::ErrorKind::Other, "can not seek pipes"))
    }
}
impl Write for Fd {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        if let Some(sender) = self.sender.as_mut() {
            let buf_len = buf.len();
            let mut buf = Some(buf.to_vec());
            loop {
                // Try and send the data
                match sender.try_send(buf.take().unwrap()) {
                    Ok(_) => { return Ok(buf_len); }
                    Err(TrySendError::Full(returned_buf)) => {
                        buf = Some(returned_buf);
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
                    wasmer::RuntimeError::raise(Box::new(wasmer_wasi::WasiError::Exit(forced_exit)));
                }

                // Maybe we are closed - if not then yield and try again
                if self.closed.load(Ordering::Acquire) {
                    return Ok(0usize);
                }
                std::thread::park_timeout(std::time::Duration::from_millis(5));
            }
        } else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Fd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(receiver) = self.receiver.as_mut() {
            loop {
                // Make an attempt to read the data
                if let Ok(mut receiver) = receiver.try_lock()
                {
                    // If we have any data then lets go!
                    if receiver.buffer.has_remaining() {
                        let max = receiver.buffer.remaining().min(buf.len());
                        buf[0..max].copy_from_slice(&receiver.buffer[..max]);
                        receiver.buffer.advance(max);
                        return Ok(max);
                    }

                    // Otherwise lets get some more data
                    match receiver.rx.try_recv() {
                        Ok(data) => {
                            receiver.buffer.extend_from_slice(&data[..]);
                            if receiver.mode == ReceiverMode::Message(false) {
                                receiver.mode = ReceiverMode::Message(true);
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
                    wasmer::RuntimeError::raise(Box::new(wasmer_wasi::WasiError::Exit(forced_exit)));
                }

                // Maybe we are closed - if not then yield and try again
                if self.closed.load(Ordering::Acquire) {
                    std::thread::yield_now();
                    return Ok(0usize);
                }
                std::thread::park_timeout(std::time::Duration::from_millis(5));
            }
        } else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
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
        Err(WasiFsError::PermissionDenied)
    }

    fn unlink(&mut self) -> Result<(), WasiFsError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WeakFd {
    pub(crate) forced_exit: Weak<AtomicU32>,
    pub(crate) closed: Weak<AtomicBool>,
    pub(crate) blocking: Weak<AtomicBool>,
    pub(crate) sender: Option<Weak<mpsc::Sender<Vec<u8>>>>,
    pub(crate) receiver: Option<Weak<AsyncMutex<ReactorPipeReceiver>>>,
}

impl WeakFd {
    pub fn upgrade(&self) -> Option<Fd> {
        let forced_exit = match self.forced_exit.upgrade() {
            Some(a) => a,
            None => { return None; }
        };

        let closed = match self.closed.upgrade() {
            Some(a) => a,
            None => { return None; }
        };

        let blocking = match self.blocking.upgrade() {
            Some(a) => a,
            None => { return None; }
        };

        let sender = self.sender.iter().filter_map(|a| a.upgrade()).next();

        let receiver = self.receiver.iter().filter_map(|a| a.upgrade()).next();

        Some(Fd {
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
            forced_exit,
            closed,
            blocking,
            sender,
            receiver,
        }
    }
}
