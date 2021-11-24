use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use serde::*;
use wasm_bus::abi::CallError;

pub trait Invokable
where Self: Send + Sync,
{
    fn send(&self, request: Vec<u8>, response: Box<dyn Fn(Result<Vec<u8>, CallError>)>);
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

impl Invokable
for ErrornousInvokable
{
    fn send(&self, _request: Vec<u8>, response: Box<dyn Fn(Result<Vec<u8>, CallError>)>) {
        let err = self.err;
        response(Err(err));
    }
}

pub struct CallbackInvokable<REQ, RES, F>
where REQ: de::DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      REQ: 'static,
      RES: 'static,
      F: Fn(REQ) -> RES,
      F: Send + Sync + 'static
{
    callback: Arc<F>,
    _marker1: PhantomData<REQ>,
    _marker2: PhantomData<RES>,
}

impl<REQ, RES, F> CallbackInvokable<REQ, RES, F>
where REQ: de::DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      REQ: 'static,
      RES: 'static,
      F: Fn(REQ) -> RES,
      F: Send + Sync + 'static
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

impl<REQ, RES, F> Invokable
for CallbackInvokable<REQ, RES, F>
where REQ: de::DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      REQ: 'static,
      RES: 'static,
      F: Fn(REQ) -> RES,
      F: Send + Sync + 'static
{
    fn send(&self, request: Vec<u8>, response: Box<dyn Fn(Result<Vec<u8>, CallError>) + 'static>) {
        let req: REQ = match bincode::deserialize(request.as_ref()) {
            Ok(a) => a,
            Err(_err) => {
                response(Err(CallError::DeserializationFailed));
                return;
            }
        };

        let callback = self.callback.deref();
        let result = callback(req);

        let result = match bincode::serialize(&result) {
            Ok(a) => a,
            Err(_err) => {
                response(Err(CallError::DeserializationFailed));
                return;
            }
        };

        response(Ok(result));
    }
}