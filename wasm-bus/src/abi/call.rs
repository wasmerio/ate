#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::marker::PhantomData;
use std::ops::*;
use serde::*;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use derivative::*;
use std::sync::Arc;
use std::sync::RwLock;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Mutex;

use super::*;

pub trait CallOps
where Self: Send + Sync
{
    fn data(&self, topic: String, data: Vec<u8>);

    fn error(&self, error: CallError);
}

pub struct CallState
{
    pub(crate) result: Option<Result<Vec<u8>, CallError>>,
    pub(crate) callbacks: HashMap<String, Mutex<Box<dyn FnMut(Vec<u8>) + Send + 'static>>>,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[must_use = "you must 'wait' or 'await' to actually send this call to other modules"]
pub struct Call
{
    pub(crate) wapm: Cow<'static, str>,
    pub(crate) topic: Cow<'static, str>,
    pub(crate) handle: CallHandle,
    #[derivative(Debug="ignore")]
    pub(crate) state: Arc<RwLock<CallState>>,
}

impl CallOps
for Call
{
    fn data(&self, topic: String, data: Vec<u8>) {
        if topic == self.topic {
            let mut state = self.state.write().unwrap();
            state.result = Some(Ok(data));
        } else {
            let state = self.state.read().unwrap();
            if let Some(callback) = state.callbacks.get(&topic) {
                let mut callback = callback.lock().unwrap();
                callback(data);
            }
        }
    }

    fn error(&self, error: CallError) {
        let mut state = self.state.write().unwrap();
        state.result = Some(Err(error));
        state.callbacks.clear();
    }
}

impl Drop
for Call
{
    fn drop(&mut self) {
        super::drop(self.handle);
        crate::engine::BusEngine::remove(&self.handle);
    }
}

impl Call
{
    pub fn id(&self) -> u32 {
        self.handle.id
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
#[must_use = "you must 'invoke' the builder for it to actually call anything"]
pub struct CallBuilder
{
    call: Call,
    request: Data,
}

impl CallBuilder
{
    pub fn new(call: Call, request: Data) -> CallBuilder {
        CallBuilder {
            call,
            request,
        }
    }
}

impl CallBuilder
{
    /// Upon receiving a particular message from the service that is
    /// invoked this callback will take some action
    /// (this function handles both synchonrous and asynchronout
    ///  callbacks - just return a Callbacklifetime using either)
    pub fn with_callback<C, F>(self, mut callback: F) -> Self
    where C: Serialize + de::DeserializeOwned + Send + 'static,
          F: FnMut(C),
          F: Send + 'static,
    {
        let handle = self.call.handle;
        super::recv_recursive::<(), C>(handle);
        let topic = std::any::type_name::<C>();

        let callback =
            move |req: Vec<u8>| {
                let req = bincode::deserialize::<C>(req.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed);
                if let Ok(req) = req {
                    callback(req);
                }
            };

        {
            let mut state = self.call.state.write().unwrap();
            state.callbacks.insert(topic.into(), Mutex::new(Box::new(callback)));
        }
        self
    }

    /// Invokes the call with the specified callbacks
    pub fn invoke(self) -> Call
    {
        match self.request {
            Data::Success(req) => {
                crate::abi::syscall::call(self.call.handle, &self.call.wapm, &self.call.topic, &req[..]);
            }
            Data::Error(err) => {
                crate::engine::BusEngine::error(self.call.handle, err);
            }
        }
        
        self.call
    }
}

impl Call
{
    /// Creates another call relative to this call
    /// This can be useful for creating contextual objects using thread calls
    /// and then passing data or commands back and forth to it
    pub fn call<T>(&self, req: T) -> CallBuilder
    where T: Serialize,
    {
        super::call_recursive(self.handle, self.wapm.clone(), req)
    }

    /// Returns the result of the call
    pub fn join<T>(self) -> CallJoin<T>
    where T: de::DeserializeOwned
    {
        CallJoin::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct CallJoin<T>
where T: de::DeserializeOwned
{
    call: Call,
    _marker1: PhantomData<T>
}

impl<T> CallJoin<T>
where T: de::DeserializeOwned
{
    fn new(call: Call) -> CallJoin<T> {
        CallJoin {
            call,
            _marker1: PhantomData
        }
    }

    /// Waits for the call to complete and returns the response from
    /// the server
    pub fn wait(self) -> Result<T, CallError>
    {
        crate::backend::block_on::block_on(self)
    }

    /// Tries to get the result of the call to the server but will not
    /// block the execution
    pub fn try_wait(&mut self) -> Result<Option<T>, CallError>
    where T: de::DeserializeOwned
    {
        let response = {            
            let mut state = self.call.state.write().unwrap();
            state.result.take()
        };

        match response {
            Some(Ok(res)) => {
                let res = bincode::deserialize::<T>(res.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed);
                match res {
                    Ok(data) => Ok(Some(data)),
                    Err(err) => Err(err)
                }
            },
            Some(Err(err)) => {
                Err(err)
            }
            None => Ok(None)
        }
    }
}

impl<T> Future
for CallJoin<T>
where T: de::DeserializeOwned
{
    type Output = Result<T, CallError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
    {
        let response = {            
            let mut state = self.call.state.write().unwrap();
            state.result.take()
        };

        match response {
            Some(Ok(response)) => {
                let res = bincode::deserialize::<T>(response.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed);
                match res {
                    Ok(data) => Poll::Ready(Ok(data)),
                    Err(err) => Poll::Ready(Err(err))
                }
            },
            Some(Err(err)) => {
                Poll::Ready(Err(err))
            }
            None => {
                crate::engine::BusEngine::subscribe(&self.call.handle, cx);
                Poll::Pending
            }
        }        
    }
}