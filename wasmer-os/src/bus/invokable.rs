use async_trait::async_trait;
use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_bus::abi::BusError;
use wasmer_bus::abi::SerializationFormat;
use wasmer_vbus::BusDataFormat;

use crate::api::AsyncResult;
use std::future::Future;
use std::pin::Pin;

pub enum InvokeResult {
    Response(SerializationFormat, Vec<u8>),
    ResponseThenWork(SerializationFormat, Vec<u8>, Pin<Box<dyn Future<Output = ()> + Send + 'static>>),
    ResponseThenLeak(SerializationFormat, Vec<u8>),
}

#[async_trait]
pub trait Processable
where
    Self: Send,
{
    async fn process(&mut self) -> Result<InvokeResult, BusError>;
}

#[async_trait]
pub trait Session
where
    Self: Send,
{
    fn call(&mut self, _topic_hash: u128, _format: BusDataFormat, _request: Vec<u8>) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        Ok((ErrornousInvokable::new(BusError::InvalidTopic), None))
    }
}

pub struct ErrornousInvokable {
    err: BusError,
}

impl ErrornousInvokable {
    pub fn new(err: BusError) -> Box<dyn Processable> {
        Box::new(ErrornousInvokable { err })
    }
}

#[async_trait]
impl Processable for ErrornousInvokable {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        Err(self.err)
    }
}

#[derive(Clone)]
pub struct ResultInvokable
where Self: Send + 'static,
{
    ret: Option<Result<Vec<u8>, BusError>>,
    format: SerializationFormat,
    leak: bool,
}

impl ResultInvokable
where Self: Send + 'static,
{
    pub fn new<T>(format: SerializationFormat, value: T) -> Box<dyn Processable>
    where T: Serialize + Send,
    {
        Self::new_strong(format, value)
    }

    pub fn new_strong<T>(format: SerializationFormat, value: T) -> Box<ResultInvokable>
    where T: Serialize + Send,
    {
        let ret = format.serialize(&value);
        Box::new(ResultInvokable {
            ret: Some(ret),
            format,
            leak: false
        })
    }

    pub fn new_leaked<T>(format: SerializationFormat, value: T) -> Box<ResultInvokable>
    where T: Serialize + Send,
    {
        let ret = format.serialize(&value);
        Box::new(ResultInvokable {
            ret: Some(ret),
            format,
            leak: true
        })
    }
}

#[async_trait]
impl Processable for ResultInvokable
where Self: Send + 'static,
{
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        if let Some(ret) = self.ret.take() {
            if self.leak {
                ret.map(|ret| {
                    InvokeResult::ResponseThenLeak(self.format, ret)
                })
            } else {
                ret.map(|ret| {
                    InvokeResult::Response(self.format, ret)
                })
            }
        } else {
            Err(BusError::AlreadyConsumed)
        }
    }
}

#[async_trait]
impl<T> Processable for AsyncResult<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        let result = self.rx.recv().await.ok_or_else(|| BusError::Aborted)?;
        Ok(InvokeResult::Response(self.format, self.format.serialize(&result)?))
    }
}
