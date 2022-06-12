use async_trait::async_trait;
use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus::abi::SerializationFormat;

use super::*;
use crate::api::AsyncResult;
use std::future::Future;
use std::pin::Pin;

pub enum InvokeResult {
    Response(Vec<u8>),
    ResponseThenWork(Vec<u8>, Pin<Box<dyn Future<Output = ()> + Send + 'static>>),
    ResponseThenLeak(Vec<u8>),
}

#[async_trait]
pub trait Invokable
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
    fn call(&mut self, _topic: &str, _request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        Ok((ErrornousInvokable::new(BusError::InvalidTopic), None))
    }
}

pub struct ErrornousInvokable {
    err: BusError,
}

impl ErrornousInvokable {
    pub fn new(err: BusError) -> Box<dyn Invokable> {
        Box::new(ErrornousInvokable { err })
    }
}

#[async_trait]
impl Invokable for ErrornousInvokable {
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        Err(self.err)
    }
}

#[derive(Clone)]
pub struct ResultInvokable
where Self: Send + 'static,
{
    ret: Option<Result<Vec<u8>, BusError>>,
    leak: bool,
}

impl ResultInvokable
where Self: Send + 'static,
{
    pub fn new<T>(format: SerializationFormat, value: T) -> Box<dyn Invokable>
    where T: Serialize + Send,
    {
        Self::new_strong(format, value)
    }

    pub fn new_strong<T>(format: SerializationFormat, value: T) -> Box<ResultInvokable>
    where T: Serialize + Send,
    {
        let ret = encode_response(
            format,
            &value,
        );
        Box::new(ResultInvokable {
            ret: Some(ret),
            leak: false
        })
    }

    pub fn new_leaked<T>(format: SerializationFormat, value: T) -> Box<ResultInvokable>
    where T: Serialize + Send,
    {
        let ret = encode_response(
            format,
            &value,
        );
        Box::new(ResultInvokable {
            ret: Some(ret),
            leak: true
        })
    }
}

#[async_trait]
impl Invokable for ResultInvokable
where Self: Send + 'static,
{
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        if let Some(ret) = self.ret.take() {
            if self.leak {
                ret.map(|ret| {
                    InvokeResult::ResponseThenLeak(ret)
                })
            } else {
                ret.map(|ret| {
                    InvokeResult::Response(ret)
                })
            }
        } else {
            Err(BusError::AlreadyConsumed)
        }
    }
}

#[async_trait]
impl<T> Invokable for AsyncResult<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        let result = self.rx.recv().await.ok_or_else(|| BusError::Aborted)?;
        Ok(InvokeResult::Response(encode_response(
            self.format,
            &result,
        )?))
    }
}
