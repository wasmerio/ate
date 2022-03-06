use async_trait::async_trait;
use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
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
    async fn process(&mut self) -> Result<InvokeResult, CallError>;
}

#[async_trait]
pub trait Session
where
    Self: Send,
{
    fn call(&mut self, _topic: &str, _request: Vec<u8>, _keepalive: bool) -> Box<dyn Invokable + 'static> {
        ErrornousInvokable::new(CallError::InvalidTopic)
    }
}

pub struct ErrornousInvokable {
    err: CallError,
}

impl ErrornousInvokable {
    pub fn new(err: CallError) -> Box<dyn Invokable> {
        Box::new(ErrornousInvokable { err })
    }
}

#[async_trait]
impl Invokable for ErrornousInvokable {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        Err(self.err)
    }
}

pub struct ResultInvokable<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    value: T,
    format: SerializationFormat,
}

impl<T> ResultInvokable<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    pub fn new(format: SerializationFormat, value: T) -> Box<dyn Invokable> {
        Box::new(ResultInvokable { value, format })
    }
}

#[async_trait]
impl<T> Invokable for ResultInvokable<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        Ok(InvokeResult::Response(encode_response(
            self.format,
            &self.value,
        )?))
    }
}

#[async_trait]
impl<T> Invokable for AsyncResult<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let result = self.rx.recv().await.ok_or_else(|| CallError::Aborted)?;
        Ok(InvokeResult::Response(encode_response(
            self.format,
            &result,
        )?))
    }
}
