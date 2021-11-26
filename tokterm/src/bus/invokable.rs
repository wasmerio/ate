#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use serde::*;
use wasm_bus::abi::CallError;
use async_trait::async_trait;
use std::future::Future;

#[async_trait]
pub trait Invokable
where Self: Send + Sync,
{
    async fn process(&self, request: Vec<u8>) -> Result<Vec<u8>, CallError>;
}

pub struct ErrornousInvokable
{
    err: CallError,
}

impl ErrornousInvokable
{
    pub fn new(err: CallError) -> Arc<dyn Invokable> {
        Arc::new(ErrornousInvokable {
            err,
        })
    }
}

#[async_trait]
impl Invokable
for ErrornousInvokable
{
    async fn process(&self, _request: Vec<u8>) -> Result<Vec<u8>, CallError> {
        let err = self.err;
        Err(err)
    }
}

pub struct CallbackInvokable<RES, REQ, F, Fut>
where REQ: de::DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      REQ: 'static,
      RES: 'static,
      F: Fn(REQ) -> Fut,
      F: Send + Sync + 'static,
      Fut: Future<Output=RES>,
      Fut: Send + Sync + 'static
{
    callback: Arc<F>,
    _marker1: PhantomData<REQ>,
    _marker2: PhantomData<RES>,
}

impl<RES, REQ, F, Fut> CallbackInvokable<RES, REQ, F, Fut>
where REQ: de::DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      REQ: 'static,
      RES: 'static,
      F: Fn(REQ) -> Fut,
      F: Send + Sync + 'static,
      Fut: Future<Output=RES>,
      Fut: Send + Sync + 'static,
{
    #[allow(dead_code)]
    pub fn new(callback: F) -> Arc<dyn Invokable>
    {
        Arc::new(CallbackInvokable {
            callback: Arc::new(callback),
            _marker1: PhantomData,
            _marker2: PhantomData
        })
    }
}

#[async_trait]
impl<RES, REQ, F, Fut> Invokable
for CallbackInvokable<RES, REQ, F, Fut>
where REQ: de::DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      REQ: 'static,
      RES: 'static,
      F: Fn(REQ) -> Fut,
      F: Send + Sync + 'static,
      Fut: Future<Output=RES>,
      Fut: Send + Sync + 'static
{
    async fn process(&self, request: Vec<u8>) -> Result<Vec<u8>, CallError> {
        let req: REQ = match bincode::deserialize(request.as_ref()) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to deserialize bus call - {}", err);
                return Err(CallError::DeserializationFailed);
            }
        };

        let callback = self.callback.deref();
        let result = callback(req).await;

        let result = match bincode::serialize(&result) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to serialize bus call response - {}", err);
                return Err(CallError::SerializationFailed);
            }
        };

        Ok(result)
    }
}