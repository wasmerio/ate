use async_trait::async_trait;
use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;

use super::*;

#[async_trait]
pub trait Invokable
where
    Self: Send,
{
    async fn process(&mut self) -> Result<Vec<u8>, CallError>;
}

pub trait Session {
    fn call(&mut self, _topic: &str, _request: &Vec<u8>) -> Box<dyn Invokable + 'static> {
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
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        Err(self.err)
    }
}

pub struct ResultInvokable<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    value: T,
}

impl<T> ResultInvokable<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    pub fn new(value: T) -> Box<dyn Invokable> {
        Box::new(ResultInvokable { value })
    }
}

#[async_trait]
impl<T> Invokable for ResultInvokable<T>
where
    Self: Send + 'static,
    T: Serialize + Send,
{
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        Ok(encode_response(&self.value)?)
    }
}
