#![allow(unused_imports, dead_code)]
use crate::common::MAX_MPSC;
use crate::fd::FdFlag;
use crate::fd::FdMsg;
use async_trait::async_trait;
use wasm_bus_fuse::fuse::FsResult;
use wasmer_vbus::BusDataFormat;
use std::any::type_name;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::ops::Deref;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus::abi::SerializationFormat;
use wasm_bus_tty::api;
use std::sync::Arc;

use super::*;
use crate::api::*;

pub fn stdin(
    _req: api::TtyStdinRequest,
    tty: crate::fs::TtyFile,
    this_callback: &Arc<dyn BusStatefulFeeder + Send + Sync + 'static>,
    mut client_callbacks: HashMap<String, Arc<dyn BusStatefulFeeder + Send + Sync + 'static>>,
) -> Result<(StdinInvoker, StdinSession), BusError> {

    // Build all the callbacks
    let on_recv = client_callbacks
        .remove(&type_name::<api::TtyStdinRecvCallback>().to_string());
    let on_flush = client_callbacks
        .remove(&type_name::<api::TtyStdinFlushCallback>().to_string());
    if on_recv.is_none() || on_flush.is_none() {
        return Err(BusError::MissingCallbacks);
    }
    let on_recv = on_recv.unwrap();
    let on_flush = on_flush.unwrap();

    // Return the invokers
    let stdin = Stdin { tty, on_recv, on_flush, this: this_callback.clone() };
    let invoker = StdinInvoker {
        format: SerializationFormat::Bincode,
        stdin: Some(stdin)
    };
    let session = StdinSession {};
    Ok((invoker, session))
}

pub struct Stdin {
    tty: crate::fs::TtyFile,
    this: Arc<dyn BusStatefulFeeder + Send + Sync + 'static>,
    on_recv: Arc<dyn BusStatefulFeeder + Send + Sync + 'static>,
    on_flush: Arc<dyn BusStatefulFeeder + Send + Sync + 'static>,
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
    format: SerializationFormat,
    stdin: Option<Stdin>,
}

#[async_trait]
impl Processable for StdinInvoker {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        let stdin = self.stdin.take();
        if let Some(stdin) = stdin {
            let fut = Box::pin(stdin.run());
            Ok(InvokeResult::ResponseThenWork(
                self.format,
                self.format.serialize(&())?,
                fut,
            ))
        } else {
            Err(BusError::Unknown)
        }
    }
}

pub struct StdinSession {
}

impl Session for StdinSession {
    fn call(&mut self, _topic_hash: u128, _format: BusDataFormat, _request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        Ok((ErrornousInvokable::new(BusError::InvalidTopic), None))
    }
}

pub fn stdout(
    _req: api::TtyStdoutRequest,
    stdout: crate::stdout::Stdout,
    _client_callbacks: HashMap<String, Arc<dyn BusStatefulFeeder + Send + Sync + 'static>>,
) -> Result<(StdoutInvoker, StdoutSession), BusError> {

    // Return the invokers
    let invoker = StdoutInvoker {
        format: SerializationFormat::Bincode
    };
    let session = StdoutSession { stdout };
    Ok((invoker, session))
}

pub struct StdoutInvoker {
    format: SerializationFormat,
}

#[async_trait]
impl Processable for StdoutInvoker {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        Ok(InvokeResult::ResponseThenLeak(
            self.format,
            self.format.serialize(&())?,
        ))
    }
}

pub struct StdoutSession {
    stdout: crate::stdout::Stdout,
}

impl Session for StdoutSession {
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        let ret = {
            if topic_hash == type_name_hash::<api::StdoutWriteRequest>() {
                let data = match conv_format(format).deserialize::<api::StdoutWriteRequest>(request) {
                    Ok(a) => a.data,
                    Err(err) => {
                        return Ok((ErrornousInvokable::new(err), None));
                    }
                };
                match self.stdout.try_write(&data[..]) {
                    Ok(Some(data_len)) => {
                        ResultInvokable::new(
                            conv_format(format),
                            api::WriteResult::Success(data_len),
                        )
                    },
                    Ok(None) => {
                        Box::new(DelayedStdoutSend {
                            format: conv_format(format),
                            data: Some(data),
                            stdout: self.stdout.clone(),
                        })
                    },
                    Err(err) => {
                        ResultInvokable::new(
                            conv_format(format),
                            api::WriteResult::Failed(err.to_string()),
                        )
                    },
                }
            } else if topic_hash == type_name_hash::<api::StdoutFlushRequest>() {
                Box::new(DelayedStdoutFlush {
                    format: conv_format(format),
                    stdout: self.stdout.clone(),
                })
            } else {
                ErrornousInvokable::new(BusError::InvalidTopic)
            }
        };
        Ok((ret, None))
    }
}

struct DelayedStdoutFlush {
    format: SerializationFormat,
    stdout: crate::stdout::Stdout,
}

#[async_trait]
impl Processable for DelayedStdoutFlush {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        let _ = self.stdout.flush_async().await;
        ResultInvokable::new(self.format, ())
            .process()
            .await
    }
}

struct DelayedStdoutSend {
    format: SerializationFormat,
    data: Option<Vec<u8>>,
    stdout: crate::stdout::Stdout,
}

#[async_trait]
impl Processable for DelayedStdoutSend {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        let mut size = 0usize;
        if let Some(data) = self.data.take() {
            size = data.len();
            if let Err(err) = self.stdout.write(&data[..]).await {
                return ResultInvokable::new(self.format, api::WriteResult::Failed(err.to_string()))
                    .process()
                    .await;
            }
        }
        ResultInvokable::new(self.format, api::WriteResult::Success(size))
            .process()
            .await
    }
}

pub fn stderr(
    _req: api::TtyStderrRequest,
    stderr: crate::fd::Fd,
    _client_callbacks: HashMap<String, Arc<dyn BusStatefulFeeder + Send + Sync + 'static>>,
) -> Result<(StderrInvoker, StderrSession), BusError> {

    // Return the invokers
    let invoker = StderrInvoker {
        format: SerializationFormat::Bincode
    };
    let session = StderrSession { stderr };
    Ok((invoker, session))
}

pub struct StderrInvoker {
    format: SerializationFormat
}

#[async_trait]
impl Processable for StderrInvoker {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        Ok(InvokeResult::ResponseThenLeak(
            self.format,
            self.format.serialize(&())?,
        ))
    }
}

pub struct StderrSession {
    stderr: crate::fd::Fd
}

impl Session for StderrSession {
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        let ret = {
            if topic_hash == type_name_hash::<api::StderrWriteRequest>() {
                let data = match conv_format(format).deserialize::<api::StderrWriteRequest>(request) {
                    Ok(a) => a.data,
                    Err(err) => {
                        return Ok((ErrornousInvokable::new(err), None));
                    }
                };
                match self.stderr.try_write(&data[..]) {
                    Ok(Some(data_len)) => {
                        ResultInvokable::new(
                            conv_format(format),
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
                            conv_format(format),
                            api::WriteResult::Failed(err.to_string()),
                        )
                    },
                }
            } else if topic_hash == type_name_hash::<api::StderrFlushRequest>() {
                Box::new(DelayedStderrFlush {
                    stderr: self.stderr.clone(),
                })
            } else {
                ErrornousInvokable::new(BusError::InvalidTopic)
            }
        };
        Ok((ret, None))
    }
}

struct DelayedStderrFlush {
    stderr: crate::fd::Fd,
}

#[async_trait]
impl Processable for DelayedStderrFlush {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
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
impl Processable for DelayedStderrSend {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
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
impl Processable for DelayedTtyRect
{
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
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
) -> Result<DelayedTtyRect, BusError> {
    Ok(DelayedTtyRect {
        abi: abi.clone()
    })
}