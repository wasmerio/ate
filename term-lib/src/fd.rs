#![allow(dead_code)]
#![allow(unused_imports)]
use bytes::{Buf, BytesMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::num::NonZeroU32;
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

use super::bus::WasmCallerContext;
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
}

impl FdFlag {
    pub fn is_tty(&self) -> bool {
        match self {
            FdFlag::Stdin(tty) => tty.clone(),
            FdFlag::Stdout(tty) => tty.clone(),
            FdFlag::Stderr(tty) => tty.clone(),
            _ => false,
        }
    }
    
    pub fn is_stdin(&self) -> bool {
        match self {
            FdFlag::Stdin(_) => true,
            _ => false,
        }
    }
}

impl std::fmt::Display
for FdFlag
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FdFlag::None => write!(f, "none"),
            FdFlag::Stdin(tty) => write!(f, "stdin(tty={})", tty),
            FdFlag::Stdout(tty) => write!(f, "stdout(tty={})", tty),
            FdFlag::Stderr(tty) => write!(f, "stderr(tty={})", tty),
            FdFlag::Log => write!(f, "log"),
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
    pub(crate) ctx: WasmCallerContext,
    pub(crate) closed: Arc<AtomicBool>,
    pub(crate) blocking: Arc<AtomicBool>,
    pub(crate) sender: Option<Arc<mpsc::Sender<FdMsg>>>,
    pub(crate) receiver: Option<Arc<AsyncMutex<ReactorPipeReceiver>>>,
    pub(crate) flip_to_abort: bool,
    pub(crate) ignore_flush: bool,
}

impl Fd {
    pub fn new(
        tx: Option<mpsc::Sender<FdMsg>>,
        rx: Option<mpsc::Receiver<FdMsg>>,
        mode: ReceiverMode,
        flag: FdFlag,
    ) -> Fd {
        let rx = rx.map(|rx| {
            Arc::new(AsyncMutex::new(ReactorPipeReceiver {
                rx,
                buffer: BytesMut::new(),
                mode,
                cur_flag: flag,
            }))
        });
        Fd {
            flag,
            ctx: WasmCallerContext::default(),
            closed: Arc::new(AtomicBool::new(false)),
            blocking: Arc::new(AtomicBool::new(true)),
            sender: tx.map(|a| Arc::new(a)),
            receiver: rx,
            flip_to_abort: false,
            ignore_flush: false,
        }
    }

    pub fn combine(fd1: &Fd, fd2: &Fd) -> Fd {
        let mut ret = Fd {
            flag: fd1.flag,
            ctx: fd1.ctx.clone(),
            closed: fd1.closed.clone(),
            blocking: Arc::new(AtomicBool::new(fd1.blocking.load(Ordering::Relaxed))),
            sender: None,
            receiver: None,
            flip_to_abort: false,
            ignore_flush: false,
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

    pub fn forced_exit(&self, exit_code: NonZeroU32) {
        self.ctx.terminate(exit_code);
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

    pub fn set_ignore_flush(&mut self, val: bool) {
        self.ignore_flush = val;
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn is_readable(&self) -> bool {
        self.receiver.is_some()
    }

    pub fn is_writable(&self) -> bool {
        self.sender.is_some()
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

    pub fn try_write(&mut self, buf: &[u8]) -> io::Result<Option<usize>> {
        self.try_send(FdMsg::new(buf.to_vec(), self.flag))
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

    pub fn poll(&self) -> PollResult {
        poll_fd(
            self.receiver.as_ref(),
            self.sender.as_ref().map(|a| a.deref()),
        )
    }

    pub async fn flush_async(&mut self) -> io::Result<()> {
        if self.ignore_flush {
            return Ok(());
        }
        let (mut rx, msg) = FdMsg::flush();
        if let Some(sender) = self.sender.as_mut() {
            if let Err(_err) = sender.send(msg).await {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
        } else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        rx.recv().await;
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
                .unwrap_or_else(|| {
                    FdMsg::new(Vec::new(), receiver.cur_flag)
                });
            if let FdMsg::Data { flag, .. } = &msg {
                receiver.cur_flag = flag.clone();
            }
            if msg.len() <= 0 {
                if self.flip_to_abort {
                    return Err(std::io::ErrorKind::BrokenPipe.into());        
                }
                self.flip_to_abort = true;
            }
            Ok(msg)
        } else {
            if self.flip_to_abort {
                return Err(std::io::ErrorKind::BrokenPipe.into());        
            }
            self.flip_to_abort = true;
            return Ok(FdMsg::new(Vec::new(), self.flag));
        }
    }

    fn try_send(&mut self, msg: FdMsg) -> io::Result<Option<usize>> {
        if let Some(sender) = self.sender.as_mut() {
            let buf_len = msg.len();

            // Try and send the data
            match sender.try_send(msg) {
                Ok(_) => {
                    return Ok(Some(buf_len));
                }
                Err(TrySendError::Closed(_)) => {
                    return Ok(Some(0));
                }
                Err(TrySendError::Full(_)) => {
                    // Check for a forced exit
                    if self.ctx.should_terminate().is_some() {
                        return Err(std::io::ErrorKind::Interrupted.into());
                    }

                    // Maybe we are closed - if not then yield and try again
                    if self.closed.load(Ordering::Acquire) {
                        return Ok(Some(0usize));
                    }

                    // We fail as this would have blocked
                    Ok(None)
                }
            }
        } else {
            return Ok(Some(0usize));
        }
    }

    fn blocking_send(&mut self, msg: FdMsg) -> io::Result<usize> {
        if let Some(sender) = self.sender.as_mut() {
            let buf_len = msg.len();

            let mut wait_time = 0u64;
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
                if self.ctx.should_terminate().is_some() {
                    return Err(std::io::ErrorKind::Interrupted.into());
                }

                // Maybe we are closed - if not then yield and try again
                if self.closed.load(Ordering::Acquire) {
                    return Ok(0usize);
                }

                // Linearly increasing wait time
                wait_time += 1;
                let wait_time = u64::min(wait_time / 10, 20);
                std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
            }
        } else {
            return Ok(0usize);
        }
    }

    fn blocking_recv<T>(&mut self, receiver: &mut mpsc::Receiver<T>) -> io::Result<Option<T>> {
        let mut tick_wait = 0u64;
        loop {
            // Try and receive the data
            match receiver.try_recv() {
                Ok(a) => {
                    return Ok(Some(a));
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    if self.flip_to_abort {
                        return Err(std::io::ErrorKind::BrokenPipe.into());        
                    }
                    self.flip_to_abort = true;
                    return Ok(None);
                }
            }

            // If we are none blocking then we are done
            if self.blocking.load(Ordering::Relaxed) == false {
                return Err(std::io::ErrorKind::WouldBlock.into());
            }

            // Check for a forced exit
            if self.ctx.should_terminate().is_some() {
                return Err(std::io::ErrorKind::Interrupted.into());
            }

            // Maybe we are closed - if not then yield and try again
            if self.closed.load(Ordering::Acquire) {
                if self.flip_to_abort {
                    return Err(std::io::ErrorKind::BrokenPipe.into());        
                }
                self.flip_to_abort = true;
                return Ok(None);
            }

            // Linearly increasing wait time
            tick_wait += 1;
            let wait_time = u64::min(tick_wait / 10, 20);
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
        if self.ignore_flush {
            return Ok(());
        }
        let (mut rx, msg) = FdMsg::flush();
        self.blocking_send(msg)?;
        self.blocking_recv(&mut rx)?;
        Ok(())
    }
}

impl Read for Fd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(receiver) = self.receiver.as_mut() {
            let mut tick_wait = 0u64;
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
                            if self.flip_to_abort {
                                return Err(std::io::ErrorKind::BrokenPipe.into());        
                            }
                            self.flip_to_abort = true;
                            return Ok(0usize);
                        }
                    }
                }

                // If we are none blocking then we are done
                if self.blocking.load(Ordering::Relaxed) == false {
                    return Err(std::io::ErrorKind::WouldBlock.into());
                }

                // Check for a forced exit
                if self.ctx.should_terminate().is_some() {
                    return Err(std::io::ErrorKind::Interrupted.into());
                }

                // Maybe we are closed - if not then yield and try again
                if self.closed.load(Ordering::Acquire) {
                    if self.flip_to_abort {
                        return Err(std::io::ErrorKind::BrokenPipe.into());        
                    }
                    self.flip_to_abort = true;
                    std::thread::yield_now();
                    return Ok(0usize);
                }

                // Linearly increasing wait time
                tick_wait += 1;
                let wait_time = u64::min(tick_wait / 10, 20);
                std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
            }
        } else {
            if self.flip_to_abort {
                return Err(std::io::ErrorKind::BrokenPipe.into());        
            }
            self.flip_to_abort = true;
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

    fn bytes_available_read(&self) -> Result<Option<usize>, WasiFsError> {
        if self.ctx.should_terminate().is_some() {
            return Err(WasiFsError::Interrupted);
        }
        let ret = self.poll();
        if ret.is_closed {
            if self.flip_to_abort {
                return Err(WasiFsError::BrokenPipe);
            }
            return Ok(Some(0usize));
        }
        if ret.can_read {
            Ok(Some(ret.bytes_available_read))
        } else {
            Ok(None)
        }
    }

    fn bytes_available_write(&self) -> Result<Option<usize>, WasiFsError> {
        if self.ctx.should_terminate().is_some() {
            return Err(WasiFsError::Interrupted);
        }
        let ret = self.poll();
        if ret.is_closed {
            if self.flip_to_abort {
                return Err(WasiFsError::BrokenPipe);
            }
            return Ok(Some(0usize));
        }
        if ret.can_write {
            Ok(Some(4096usize))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeakFd {
    pub(crate) flag: FdFlag,
    pub(crate) ctx: WasmCallerContext,
    pub(crate) closed: Weak<AtomicBool>,
    pub(crate) blocking: Weak<AtomicBool>,
    pub(crate) sender: Option<Weak<mpsc::Sender<FdMsg>>>,
    pub(crate) receiver: Option<Weak<AsyncMutex<ReactorPipeReceiver>>>,
    pub(crate) flip_to_abort: bool,
    pub(crate) ignore_flush: bool,
}

impl WeakFd {
    pub fn null() -> WeakFd {
        WeakFd {
            flag: FdFlag::None,
            ctx: WasmCallerContext::default(),
            closed: Weak::new(),
            blocking: Weak::new(),
            sender: None,
            receiver: None,
            flip_to_abort: false,
            ignore_flush: false,
        }
    }

    pub fn upgrade(&self) -> Option<Fd> {
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
            ctx: self.ctx.clone(),
            closed,
            blocking,
            sender,
            receiver,
            flip_to_abort: self.flip_to_abort,
            ignore_flush: self.ignore_flush,
        })
    }
}

impl Fd {
    pub fn downgrade(&self) -> WeakFd {
        let closed = Arc::downgrade(&self.closed);
        let blocking = Arc::downgrade(&self.blocking);
        let sender = self.sender.iter().map(|a| Arc::downgrade(&a)).next();
        let receiver = self.receiver.iter().map(|a| Arc::downgrade(&a)).next();

        WeakFd {
            flag: self.flag,
            ctx: self.ctx.clone(),
            closed,
            blocking,
            sender,
            receiver,
            flip_to_abort: self.flip_to_abort,
            ignore_flush: self.ignore_flush,
        }
    }
}
