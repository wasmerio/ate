use async_trait::async_trait;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;

#[async_trait]
pub trait Invokable
where Self: Send
{
    async fn process(&mut self) -> Result<Vec<u8>, CallError>;
}

pub trait Session
{
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