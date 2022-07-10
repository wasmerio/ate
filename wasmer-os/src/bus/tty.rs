#![allow(unused_imports, dead_code)]
use crate::common::MAX_MPSC;
use crate::fd::FdFlag;
use crate::fd::FdMsg;
use async_trait::async_trait;
use wasmer_bus_fuse::fuse::FsResult;
use wasmer_vbus::BusDataFormat;
use wasmer_vbus::BusInvocationEvent;
use wasmer_vbus::InstantInvocation;
use wasmer_vbus::VirtualBusError;
use wasmer_vbus::VirtualBusInvocation;
use wasmer_vbus::VirtualBusInvokable;
use wasmer_vbus::VirtualBusInvoked;
use std::any::type_name;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::ops::Deref;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};
use wasmer_bus::abi::BusError;
use wasmer_bus::abi::SerializationFormat;
use wasmer_bus_tty::api;
use std::sync::Arc;

use super::*;
use crate::api::*;

pub fn stdin(
    tty: crate::fs::TtyFile,
) -> Box<dyn VirtualBusInvoked> {

    // Return the invokers
    let stdin = StdinHandler { tty };
    Box::new(InstantInvocation::call(Box::new(stdin)))
}

#[derive(Debug)]
pub struct StdinHandler {
    tty: crate::fs::TtyFile,
}

impl VirtualBusInvocation
for StdinHandler {
    fn poll_event(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<BusInvocationEvent> {
        loop {
            let tty = Pin::new(&mut self.tty);
            return match tty.poll_read(cx) {
                Poll::Ready(Ok(msg)) => {
                    match msg {
                        FdMsg::Data { data, flag } => {
                            if flag.is_stdin() {
                                Poll::Ready(BusInvocationEvent::Callback {
                                    topic_hash: type_name_hash::<api::TtyStdinRecvCallback>(),
                                    format: BusDataFormat::Bincode,
                                    data: match SerializationFormat::Bincode.serialize(api::TtyStdinRecvCallback(data)) {
                                        Ok(data) => data,
                                        Err(_) => {
                                            return Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::Serialization });
                                        }
                                    }
                                })
                            } else {
                                continue;
                            }
                        }
                        FdMsg::Flush { .. } => {
                            Poll::Ready(BusInvocationEvent::Callback {
                                topic_hash: type_name_hash::<api::TtyStdinFlushCallback>(),
                                format: BusDataFormat::Bincode,
                                data: match SerializationFormat::Bincode.serialize(api::TtyStdinFlushCallback(())) {
                                    Ok(data) => data,
                                    Err(_) => {
                                        return Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::Serialization });
                                    }
                                }
                            })
                        }
                    }
                },
                Poll::Ready(Err(err)) => {
                    debug!("failed to read tty - {}", err);
                    return Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::InternalError });
                }
                Poll::Pending => Poll::Pending
            }
        }
    }
}

impl VirtualBusInvokable
for StdinHandler {
    fn invoke(
        &self,
        _topic_hash: u128,
        _format: BusDataFormat,
        _buf: Vec<u8>,
    ) -> Box<dyn VirtualBusInvoked> {
        Box::new(InstantInvocation::fault(VirtualBusError::InvalidTopic))
    }
}

pub fn stdout(
    system: System,
    stdout: crate::fd::Fd,
) -> Box<dyn VirtualBusInvoked> {

    // Return the invokers
    let handler = StdoutHandler {
        system,
        stdout,
    };
    Box::new(InstantInvocation::call(Box::new(handler)))
}

#[derive(Debug)]
pub struct StdoutHandler {
    system: System,
    stdout: crate::fd::Fd
}

impl VirtualBusInvocation
for StdoutHandler {
    fn poll_event(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<BusInvocationEvent> {
        // this call never closes by itself (only the client can close it)
        Poll::Pending
    }
}

impl VirtualBusInvokable
for StdoutHandler {
    fn invoke(
        &self,
        topic_hash: u128,
        format: BusDataFormat,
        buf: Vec<u8>,
    ) -> Box<dyn VirtualBusInvoked> {
        let mut stdout = self.stdout.clone();
        if topic_hash == type_name_hash::<api::StdoutWriteRequest>() {
            let data = match decode_request::<api::StdoutWriteRequest>(
                format,
                buf,
            ) {
                Ok(a) => a.data,
                Err(err) => {
                    return Box::new(InstantInvocation::fault(conv_error_back(err)));
                }
            };
            return match stdout.try_write(&data) {
                Ok(Some(amt)) => {
                    Box::new(encode_instant_response(BusDataFormat::Bincode, &api::WriteResult::Success(amt)))
                },
                Ok(None) => {
                    Box::new(InstantInvocation::call(
                        Box::new(self.system.spawn_shared(move || async move {
                            match stdout.write_all(&data) {
                                Ok(_) => api::WriteResult::Success(data.len()),
                                Err(err) => api::WriteResult::Failed(err.to_string())
                            }
                        }))
                    ))
                },
                Err(err) => {
                    Box::new(encode_instant_response(BusDataFormat::Bincode, &api::WriteResult::Failed(err.to_string())))
                }
            }
        } else if topic_hash == type_name_hash::<api::StdoutFlushRequest>() {
            let _ = stdout.flush();
            Box::new(encode_instant_response(BusDataFormat::Bincode, &()))
        } else {
            debug!("stdout invalid topic (hash={})", topic_hash);
            Box::new(InstantInvocation::fault(VirtualBusError::InvalidTopic))
        }
    }
}

pub fn stderr(
    system: System,
    stderr: crate::fd::Fd,
) -> Box<dyn VirtualBusInvoked> {

    // Return the invokers
    let handler = StderrHandler {
        system,
        stderr,
    };
    Box::new(InstantInvocation::call(Box::new(handler)))
}

#[derive(Debug)]
pub struct StderrHandler {
    system: System,
    stderr: crate::fd::Fd
}

impl VirtualBusInvocation
for StderrHandler {
    fn poll_event(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<BusInvocationEvent> {
        // this call never closes by itself (only the client can close it)
        Poll::Pending
    }
}

impl VirtualBusInvokable
for StderrHandler {
    fn invoke(
        &self,
        topic_hash: u128,
        format: BusDataFormat,
        buf: Vec<u8>,
    ) -> Box<dyn VirtualBusInvoked> {
        let mut stderr = self.stderr.clone();
        if topic_hash == type_name_hash::<api::StderrWriteRequest>() {
            let data = match decode_request::<api::StderrWriteRequest>(
                format,
                buf,
            ) {
                Ok(a) => a.data,
                Err(err) => {
                    return Box::new(InstantInvocation::fault(conv_error_back(err)));
                }
            };
            return match stderr.try_write(&data) {
                Ok(Some(amt)) => {
                    Box::new(encode_instant_response(BusDataFormat::Bincode, &api::WriteResult::Success(amt)))
                },
                Ok(None) => {
                    Box::new(InstantInvocation::call(
                        Box::new(self.system.spawn_shared(move || async move {
                            match stderr.write_all(&data) {
                                Ok(_) => api::WriteResult::Success(data.len()),
                                Err(err) => api::WriteResult::Failed(err.to_string())
                            }
                        }))
                    ))
                },
                Err(err) => {
                    Box::new(encode_instant_response(BusDataFormat::Bincode, &api::WriteResult::Failed(err.to_string())))
                }
            }
        } else if topic_hash == type_name_hash::<api::StderrFlushRequest>() {
            let _ = stderr.flush();
            Box::new(encode_instant_response(BusDataFormat::Bincode, &()))
        } else {
            debug!("stderr invalid topic (hash={})", topic_hash);
            Box::new(InstantInvocation::fault(VirtualBusError::InvalidTopic))
        }
    }
}

pub fn rect(
    system: System,
    abi: &Arc<dyn ConsoleAbi>,
) -> Box<dyn VirtualBusInvoked> {
    let abi = abi.clone();
    let result = system.spawn_shared(move || {
        let abi = abi.clone();
        async move {
            let rect = abi.console_rect().await;
            api::TtyRect {
                cols: rect.cols as u32,
                rows: rect.rows as u32
            }
        }
    });
    Box::new(InstantInvocation::call(Box::new(result)))
}
