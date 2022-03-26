#![allow(unused_imports, dead_code)]
use crate::common::MAX_MPSC;
use crate::fd::FdFlag;
use crate::fd::FdMsg;
use async_trait::async_trait;
use wasm_bus_fuse::fuse::FsResult;
use std::any::type_name;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::ops::Deref;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::SerializationFormat;
use wasm_bus_tty::api;
use std::sync::Arc;

use super::*;
use crate::api::*;

pub fn stdin(
    _req: api::TtyStdinRequest,
    tty: crate::fs::TtyFile,
    this_callback: &Arc<dyn BusFeeder + Send + Sync + 'static>,
    mut client_callbacks: HashMap<String, Arc<dyn BusFeeder + Send + Sync + 'static>>,
) -> Result<(StdinInvoker, StdinSession), CallError> {

    // Build all the callbacks
    let on_recv = client_callbacks
        .remove(&type_name::<api::TtyStdinRecvCallback>().to_string());
    let on_flush = client_callbacks
        .remove(&type_name::<api::TtyStdinFlushCallback>().to_string());
    if on_recv.is_none() || on_flush.is_none() {
        return Err(CallError::MissingCallbacks);
    }
    let on_recv = on_recv.unwrap();
    let on_flush = on_flush.unwrap();

    // Return the invokers
    let stdin = Stdin { tty, on_recv, on_flush, this: this_callback.clone() };
    let invoker = StdinInvoker { stdin: Some(stdin) };
    let session = StdinSession {};
    Ok((invoker, session))
}

pub struct Stdin {
    tty: crate::fs::TtyFile,
    this: Arc<dyn BusFeeder + Send + Sync + 'static>,
    on_recv: Arc<dyn BusFeeder + Send + Sync + 'static>,
    on_flush: Arc<dyn BusFeeder + Send + Sync + 'static>,
}

impl Stdin
{
    pub async fn run(mut self) {
        while let Ok(msg) = self.tty.read_async().await
        {
            match msg {
                FdMsg::Data { data, flag } => {
                    if flag.is_stdin() {
                        BusFeederUtils::feed(self.on_recv.deref(), SerializationFormat::Bincode, data);
                    }
                }
                FdMsg::Flush { .. } => {
                    BusFeederUtils::feed(self.on_flush.deref(), SerializationFormat::Bincode, ());
                }
            }
        }
        self.on_recv.terminate();
        self.on_flush.terminate();
        self.this.terminate();
    }
}

pub struct StdinInvoker
{
    stdin: Option<Stdin>,
}

#[async_trait]
impl Invokable for StdinInvoker {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let stdin = self.stdin.take();
        if let Some(stdin) = stdin {
            let fut = Box::pin(stdin.run());
            Ok(InvokeResult::ResponseThenWork(
                encode_response(SerializationFormat::Bincode, &())?,
                fut,
            ))
        } else {
            Err(CallError::Unknown)
        }
    }
}

pub struct StdinSession {
}

impl Session for StdinSession {
    fn call(&mut self, _topic: &str, _request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        Ok((ErrornousInvokable::new(CallError::InvalidTopic), None))
    }
}

pub fn stdout(
    _req: api::TtyStdoutRequest,
    stdout: crate::stdout::Stdout,
    _client_callbacks: HashMap<String, Arc<dyn BusFeeder + Send + Sync + 'static>>,
) -> Result<(StdoutInvoker, StdoutSession), CallError> {

    // Return the invokers
    let invoker = StdoutInvoker {};
    let session = StdoutSession { stdout };
    Ok((invoker, session))
}

pub struct StdoutInvoker {
}

#[async_trait]
impl Invokable for StdoutInvoker {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        Ok(InvokeResult::ResponseThenLeak(
            encode_response(SerializationFormat::Bincode, &())?,
        ))
    }
}

pub struct StdoutSession {
    stdout: crate::stdout::Stdout,
}

impl Session for StdoutSession {
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        let ret = {
            if topic == type_name::<api::StdoutWriteRequest>() {
                let data = match decode_request::<api::StdoutWriteRequest>(
                    SerializationFormat::Bincode,
                    request.as_ref(),
                ) {
                    Ok(a) => a.data,
                    Err(err) => {
                        return Ok((ErrornousInvokable::new(err), None));
                    }
                };
                match self.stdout.try_write(&data[..]) {
                    Ok(Some(data_len)) => {
                        ResultInvokable::new(
                            SerializationFormat::Bincode,
                            api::WriteResult::Success(data_len),
                        )
                    },
                    Ok(None) => {
                        Box::new(DelayedStdoutSend {
                            data: Some(data),
                            stdout: self.stdout.clone(),
                        })
                    },
                    Err(err) => {
                        ResultInvokable::new(
                            SerializationFormat::Bincode,
                            api::WriteResult::Failed(err.to_string()),
                        )
                    },
                }
            } else if topic == type_name::<api::StdoutFlushRequest>() {
                Box::new(DelayedStdoutFlush {
                    stdout: self.stdout.clone(),
                })
            } else {
                ErrornousInvokable::new(CallError::InvalidTopic)
            }
        };
        Ok((ret, None))
    }
}

struct DelayedStdoutFlush {
    stdout: crate::stdout::Stdout,
}

#[async_trait]
impl Invokable for DelayedStdoutFlush {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let _ = self.stdout.flush_async().await;
        ResultInvokable::new(SerializationFormat::Bincode, ())
            .process()
            .await
    }
}

struct DelayedStdoutSend {
    data: Option<Vec<u8>>,
    stdout: crate::stdout::Stdout,
}

#[async_trait]
impl Invokable for DelayedStdoutSend {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let mut size = 0usize;
        if let Some(data) = self.data.take() {
            size = data.len();
            if let Err(err) = self.stdout.write(&data[..]).await {
                return ResultInvokable::new(SerializationFormat::Bincode, api::WriteResult::Failed(err.to_string()))
                    .process()
                    .await;
            }
        }
        ResultInvokable::new(SerializationFormat::Bincode, api::WriteResult::Success(size))
            .process()
            .await
    }
}

pub fn stderr(
    _req: api::TtyStderrRequest,
    stderr: crate::fd::Fd,
    _client_callbacks: HashMap<String, Arc<dyn BusFeeder + Send + Sync + 'static>>,
) -> Result<(StderrInvoker, StderrSession), CallError> {

    // Return the invokers
    let invoker = StderrInvoker {};
    let session = StderrSession { stderr };
    Ok((invoker, session))
}

pub struct StderrInvoker {
}

#[async_trait]
impl Invokable for StderrInvoker {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        Ok(InvokeResult::ResponseThenLeak(
            encode_response(SerializationFormat::Bincode, &())?,
        ))
    }
}

pub struct StderrSession {
    stderr: crate::fd::Fd
}

impl Session for StderrSession {
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        let ret = {
            if topic == type_name::<api::StderrWriteRequest>() {
                let data = match decode_request::<api::StderrWriteRequest>(
                    SerializationFormat::Bincode,
                    request.as_ref(),
                ) {
                    Ok(a) => a.data,
                    Err(err) => {
                        return Ok((ErrornousInvokable::new(err), None));
                    }
                };
                match self.stderr.try_write(&data[..]) {
                    Ok(Some(data_len)) => {
                        ResultInvokable::new(
                            SerializationFormat::Bincode,
                            api::WriteResult::Success(data_len),
                        )
                    },
                    Ok(None) => {
                        Box::new(DelayedStderrSend {
                            data: Some(data),
                            stderr: self.stderr.clone(),
                        })
                    },
                    Err(err) => {
                        ResultInvokable::new(
                            SerializationFormat::Bincode,
                            api::WriteResult::Failed(err.to_string()),
                        )
                    },
                }
            } else if topic == type_name::<api::StderrFlushRequest>() {
                Box::new(DelayedStderrFlush {
                    stderr: self.stderr.clone(),
                })
            } else {
                ErrornousInvokable::new(CallError::InvalidTopic)
            }
        };
        Ok((ret, None))
    }
}

struct DelayedStderrFlush {
    stderr: crate::fd::Fd,
}

#[async_trait]
impl Invokable for DelayedStderrFlush {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let _ = self.stderr.flush_async().await;
        ResultInvokable::new(SerializationFormat::Bincode, ())
            .process()
            .await
    }
}

struct DelayedStderrSend {
    data: Option<Vec<u8>>,
    stderr: crate::fd::Fd,
}

#[async_trait]
impl Invokable for DelayedStderrSend {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let mut size = 0usize;
        if let Some(data) = self.data.take() {
            size = data.len();
            if let Err(err) = self.stderr.write(&data[..]).await {
                return ResultInvokable::new(SerializationFormat::Bincode, api::WriteResult::Failed(err.to_string()))
                    .process()
                    .await;
            }
        }
        ResultInvokable::new(SerializationFormat::Bincode, api::WriteResult::Success(size))
            .process()
            .await
    }
}

pub struct DelayedTtyRect {
    abi: Arc<dyn ConsoleAbi>
}

#[async_trait]
impl Invokable for DelayedTtyRect
{
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let rect = self.abi.console_rect().await;
        ResultInvokable::new(SerializationFormat::Bincode, api::TtyRect {
            cols: rect.cols as u32,
            rows: rect.rows as u32
        })
        .process()
        .await
    }
}

pub fn rect(
    _req: api::TtyRectRequest,
    abi: &Arc<dyn ConsoleAbi>,
) -> Result<DelayedTtyRect, CallError> {
    Ok(DelayedTtyRect {
        abi: abi.clone()
    })
}