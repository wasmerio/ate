use derivative::*;
use serde::*;
use std::borrow::Cow;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::*;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;

pub trait CallOps
where
    Self: Send + Sync,
{
    fn data(&self, data: Vec<u8>);

    fn error(&self, error: CallError);
}

pub struct CallState {
    pub(crate) result: Option<Result<Vec<u8>, CallError>>,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[must_use = "you must 'wait' or 'await' to actually send this call to other modules"]
pub struct Call {
    pub(crate) wapm: Cow<'static, str>,
    pub(crate) topic: Cow<'static, str>,
    pub(crate) format: SerializationFormat,
    pub(crate) session: Option<String>,
    pub(crate) handle: CallHandle,
    pub(crate) parent: Option<CallHandle>,
    #[derivative(Debug = "ignore")]
    pub(crate) state: Arc<Mutex<CallState>>,
    pub(crate) callbacks: Arc<Mutex<Vec<Finish>>>,
}

impl CallOps for Call {
    fn data(&self, data: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        state.result = Some(Ok(data));
    }

    fn error(&self, error: CallError) {
        let mut state = self.state.lock().unwrap();
        state.result = Some(Err(error));
    }
}

impl Drop for Call {
    fn drop(&mut self) {
        super::drop(self.handle);
        crate::engine::BusEngine::remove(&self.handle);
    }
}

impl Call {
    pub fn id(&self) -> u32 {
        self.handle.id
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
#[must_use = "you must 'invoke' the builder for it to actually call anything"]
pub struct CallBuilder {
    call: Option<Call>,
    request: Data,
}

impl CallBuilder {
    pub fn new(call: Call, request: Data) -> CallBuilder {
        CallBuilder {
            call: Some(call),
            request,
        }
    }
}

impl CallBuilder {
    /// Upon receiving a particular message from the service that is
    /// invoked this callback will take some action
    pub fn callback<C, F>(mut self, format: SerializationFormat, callback: F) -> Self
    where
        C: Serialize + de::DeserializeOwned + Send + Sync + 'static,
        F: FnMut(C),
        F: Send + 'static,
    {
        self.call.as_mut().unwrap().callback(format, callback);
        self
    }

    /// Invokes the call with the specified callbacks
    #[cfg(target_arch = "wasm32")]
    pub fn invoke(mut self) -> Call {
        let call = self.call.take().unwrap();
        match &self.request {
            Data::Success(req) => {
                crate::abi::syscall::call(
                    call.parent,
                    call.handle,
                    &call.wapm,
                    &call.topic,
                    &req[..],
                );
            }
            Data::Error(err) => {
                crate::engine::BusEngine::error(call.handle, err.clone());
            }
        }

        call
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn invoke(self) -> Call {
        panic!("invoke not supported on this platform");
    }
}

impl Drop for CallBuilder {
    fn drop(&mut self) {
        if let Some(call) = self.call.take() {
            super::drop(call.handle);
            crate::engine::BusEngine::remove(&call.handle);
        }
    }
}

impl Call {
    /// Creates another call relative to this call
    /// This can be useful for creating contextual objects using thread calls
    /// and then passing data or commands back and forth to it
    pub fn call<T>(
        &self,
        format: SerializationFormat,
        session: Option<String>,
        req: T,
    ) -> CallBuilder
    where
        T: Serialize,
    {
        super::call_internal(Some(self.handle), self.wapm.clone(), format, session, req)
    }

    /// Upon receiving a particular message from the service that is
    /// invoked this callback will take some action
    ///
    /// Note: This must be called before the invoke or things will go wrong
    /// hence there is a builder that invokes this in the right order
    fn callback<C, F>(&mut self, format: SerializationFormat, mut callback: F) -> &mut Self
    where
        C: Serialize + de::DeserializeOwned + Send + Sync + 'static,
        F: FnMut(C),
        F: Send + 'static,
    {
        let callback = move |req| {
            callback(req);
            Ok(())
        };
        let recv = super::callback_internal(self.handle, format, callback);
        self.callbacks.lock().unwrap().push(recv);
        self
    }

    /// Returns the result of the call
    pub fn join<T>(self) -> CallJoin<T>
    where
        T: de::DeserializeOwned,
    {
        CallJoin::new(self)
    }
}

#[derive(Debug, Clone)]
#[must_use = "this `Call` only does something when you consume it"]
pub struct CallJoin<T>
where
    T: de::DeserializeOwned,
{
    call: Call,
    _marker1: PhantomData<T>,
}

impl<T> CallJoin<T>
where
    T: de::DeserializeOwned,
{
    fn new(call: Call) -> CallJoin<T> {
        CallJoin {
            call,
            _marker1: PhantomData,
        }
    }

    /// Waits for the call to complete and returns the response from
    /// the server
    #[cfg(feature = "rt")]
    pub fn wait(self) -> Result<T, CallError> {
        crate::task::block_on(self)
    }

    /// Spawns the work on a background thread
    #[cfg(feature = "rt")]
    pub fn spawn(self)
    where
        T: Send + 'static,
    {
        crate::task::spawn(self);
    }

    /// Tries to get the result of the call to the server but will not
    /// block the execution
    pub fn try_wait(&mut self) -> Result<Option<T>, CallError>
    where
        T: de::DeserializeOwned,
    {
        let response = {
            let mut state = self.call.state.lock().unwrap();
            state.result.take()
        };

        match response {
            Some(Ok(res)) => {
                let res = match self.call.format {
                    SerializationFormat::Bincode => bincode::deserialize::<T>(res.as_ref())
                        .map_err(|_err| CallError::DeserializationFailed),
                    SerializationFormat::Json => serde_json::from_slice(res.as_ref())
                        .map_err(|_err| CallError::DeserializationFailed),
                };
                match res {
                    Ok(data) => Ok(Some(data)),
                    Err(err) => Err(err),
                }
            }
            Some(Err(err)) => Err(err),
            None => Ok(None),
        }
    }
}

impl<T> Future for CallJoin<T>
where
    T: de::DeserializeOwned,
{
    type Output = Result<T, CallError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let response = {
            let mut state = self.call.state.lock().unwrap();
            state.result.take()
        };

        match response {
            Some(Ok(response)) => {
                let res = match self.call.format {
                    SerializationFormat::Bincode => bincode::deserialize::<T>(response.as_ref())
                        .map_err(|_err| CallError::DeserializationFailed),
                    SerializationFormat::Json => serde_json::from_slice(response.as_ref())
                        .map_err(|_err| CallError::DeserializationFailed),
                };
                match res {
                    Ok(data) => Poll::Ready(Ok(data)),
                    Err(err) => Poll::Ready(Err(err)),
                }
            }
            Some(Err(err)) => Poll::Ready(Err(err)),
            None => {
                crate::engine::BusEngine::subscribe(&self.call.handle, cx);
                Poll::Pending
            }
        }
    }
}
