use derivative::*;
use serde::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::*;
use std::pin::Pin;
use std::sync::atomic::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallbackResult {
    InvalidTopic,
    Success,
    Error,
}

pub trait CallOps
where
    Self: Send + Sync,
{
    fn data(&self, data: Vec<u8>, format: SerializationFormat);

    fn callback(&self, topic: Cow<'static, str>, data: Vec<u8>, format: SerializationFormat) -> CallbackResult;

    fn error(&self, error: BusError);

    fn topic(&self) -> &str;
}

#[derive(Debug)]
pub(crate) struct CallResult
{
    pub data: Vec<u8>,
    pub format: SerializationFormat,
}

#[derive(Debug)]
pub struct CallState {
    pub(crate) result: Option<Result<CallResult, BusError>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallInstance {
    pub(crate) instance: Cow<'static, str>,
    pub(crate) access_token: Cow<'static, str>,
}

impl CallInstance
{
    pub fn new(instance: &str, access_token: &str) -> CallInstance {
        CallInstance { 
            instance: instance.to_string().into(),
            access_token: access_token.to_string().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallContext {
    NewBusCall {
        wapm: Cow<'static, str>,
        instance: Option<CallInstance>,
    },
    SubCall {
        parent: CallSmartHandle,
    }
}

/// When this object is destroyed it will kill the call on the bus
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallSmartHandle {
    inside: Arc<CallSmartPointerInside>,
}

impl CallSmartHandle {
    pub fn new(handle: CallHandle) -> CallSmartHandle {
        CallSmartHandle {
            inside: Arc::new(
                CallSmartPointerInside {
                    handle
                }
            )
        }
    }

    pub fn cid(&self) -> CallHandle {
        self.inside.handle
    }
}

/// When this object is destroyed it will kill the call on the bus
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CallSmartPointerInside {
    handle: CallHandle,
}

impl Drop
for CallSmartPointerInside {
    fn drop(&mut self) {
        crate::abi::syscall::call_close(self.handle);
        crate::engine::BusEngine::close(&self.handle, "call closed");
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[must_use = "you must 'wait' or 'await' to actually send this call to other modules"]
pub struct Call {
    pub(crate) ctx: CallContext,
    pub(crate) topic: Cow<'static, str>,
    #[derivative(Debug = "ignore")]
    pub(crate) callbacks: HashMap<Cow<'static, str>, Arc<dyn Fn(Vec<u8>, SerializationFormat) -> CallbackResult + Send + Sync + 'static>>,
    pub(crate) handle: Option<CallSmartHandle>,
    pub(crate) keepalive: bool,
    pub(crate) state: Arc<Mutex<CallState>>,
    pub(crate) drop_on_data: Arc<AtomicBool>,
}

impl Call {
    pub fn new_call(
        wapm: Cow<'static, str>,
        topic: Cow<'static, str>,
        instance: Option<CallInstance>,
    ) -> Call {
        Call {
            ctx: CallContext::NewBusCall { wapm, instance },
            state: Arc::new(Mutex::new(CallState {
                result: None,
            })),
            callbacks: Default::default(),
            handle: None,
            keepalive: false,
            topic,
            drop_on_data: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn new_subcall(
        parent: CallSmartHandle,
        topic: Cow<'static, str>,
    ) -> Call {
        Call {
            ctx: CallContext::SubCall { parent },
            state: Arc::new(Mutex::new(CallState {
                result: None,
            })),
            callbacks: Default::default(),
            handle: None,
            keepalive: false,
            topic,
            drop_on_data: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn handle(&self) -> Option<CallSmartHandle> {
        self.handle.clone()
    }
}

impl CallOps for Call {
    fn data(&self, data: Vec<u8>, format: SerializationFormat) {
        {
            let mut state = self.state.lock().unwrap();
            state.result = Some(Ok(CallResult {
                data,
                format
            }));
        }
        if self
            .drop_on_data
            .compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
            .is_ok()
        {
            if let Some(scope) = self.handle.as_ref() {
                super::syscall::call_close(scope.cid());
                crate::engine::BusEngine::close(&scope.cid(), "call has finished (with data)");
            }
        }
    }

    fn callback(&self, topic: Cow<'static, str>, data: Vec<u8>, format: SerializationFormat) -> CallbackResult {
        if let Some(funct) = self.callbacks.get(&topic) {
            funct(data, format)
        } else {
            CallbackResult::InvalidTopic
        }
    }

    fn error(&self, error: BusError) {
        {
            let mut state = self.state.lock().unwrap();
            state.result = Some(Err(error));
        }
        if let Some(scope) = self.handle.as_ref() {
            crate::engine::BusEngine::close(&scope.cid(), "call has failed");
        }
    }

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
#[must_use = "you must 'invoke' the builder for it to actually call anything"]
pub struct CallBuilder {
    call: Option<Call>,
    format: SerializationFormat,
    request: Data,
}

impl CallBuilder {
    pub fn new(call: Call, request: Data, format: SerializationFormat) -> CallBuilder {
        CallBuilder {
            call: Some(call),
            format,
            request,
        }
    }
}

impl CallBuilder {
    /// Upon receiving a particular message from the service that is
    /// invoked this callback will take some action
    pub fn callback<C, F>(mut self, callback: F) -> Self
    where
        C: Serialize + de::DeserializeOwned + Send + Sync + 'static,
        F: Fn(C),
        F: Send + Sync + 'static,
    {
        self.call
            .as_mut()
            .unwrap()
            .register_callback(callback);
        self
    }

    // Completes the call but leaves the resources present
    // for the duration of the life of 'DetachedCall'
    pub async fn detach<T>(mut self) -> Result<DetachedCall<T>, BusError>
    where
        T: de::DeserializeOwned,
    {
        let mut call = self.call.take().unwrap();
        call.keepalive = true;
        call.drop_on_data.store(false, Ordering::Release);
        let scope = self.invoke_internal(&mut call)?;
        
        let result = call.join::<T>()?
            .await?;

        Ok(DetachedCall {
            handle: scope,
            result,
        })
    }

    /// Invokes the call with the specified callbacks
    pub fn invoke(mut self) -> Call {
        let mut call = self.call.take().unwrap();
        match self.invoke_internal(&mut call) {
            Ok(scope) => {
                call.handle.replace(scope);
            },
            Err(err) => {
                call.error(err);
            }
        }        
        call
    }

    fn invoke_internal(&self, call: &mut Call) -> Result<CallSmartHandle, BusError> {
        let handle = match &self.request {
            Data::Prepared(req) => {
                match &call.ctx {
                    CallContext::NewBusCall { wapm, instance } => {
                        if let Some(ref instance) = instance {
                            crate::abi::syscall::bus_open_remote(
                                wapm.as_ref(),
                                true,
                                &instance.instance,
                                &instance.access_token,
                            )
                        } else {
                            crate::abi::syscall::bus_open_local(
                                wapm.as_ref(),
                                true,
                            )
                        }.and_then(|bid| {
                            crate::abi::syscall::bus_call(
                                bid,
                                call.keepalive,
                                &call.topic,
                                &req[..],
                                self.format,
                            )
                        })
                    },
                    CallContext::SubCall { parent } => {
                        crate::abi::syscall::bus_subcall(
                            parent.cid(),
                            call.keepalive,
                            &call.topic,
                            &req[..],
                            self.format,
                        )
                    }
                }
            }
            Data::Error(err) => {
                return Err(err.clone());
            }
        }
        .map(|a| CallSmartHandle::new(a));

        if let Ok(scope) = &handle {
            let mut state = crate::engine::BusEngine::write();
            state.handles.insert(scope.cid());
            state.calls.insert(scope.cid(), Arc::new(call.clone()));
            call.handle.replace(scope.clone());
        }
        handle
    }
}

impl Call {
    /// Upon receiving a particular message from the service that is
    /// invoked this callback will take some action
    ///
    /// Note: This must be called before the invoke or things will go wrong
    /// hence there is a builder that invokes this in the right order
    fn register_callback<C, F>(&mut self, callback: F)
    where
        C: Serialize + de::DeserializeOwned + Send + Sync + 'static,
        F: Fn(C),
        F: Send + Sync + 'static,
    {
        let topic = std::any::type_name::<C>();
        let callback = move |data, format: SerializationFormat| {
            if let Ok(data) = format.deserialize(data) {
                callback(data);
                CallbackResult::Success
            } else {
                debug!("deserialization failed during callback (format={}, topic={})", format, topic);
                CallbackResult::Error
            }
        };
        self.callbacks.insert(topic.into(), Arc::new(callback));
    }

    /// Returns the result of the call
    pub fn join<T>(self) -> Result<CallJoin<T>, BusError>
    where
        T: de::DeserializeOwned,
    {
        match self.handle.clone() {
            Some(scope) => {
                Ok(CallJoin::new(self, scope))
            },
            None => {
                trace!("must invoke the call before attempting to join on it (topic={})", self.topic());
                Err(BusError::BusInvocationFailed)
            }
        }
    }
}

#[derive(Debug)]
pub struct DetachedCall<T> {
    handle: CallSmartHandle,
    result: T,
}

impl<T> DetachedCall<T> {
    /// Creates another call relative to this call
    /// This can be useful for creating contextual objects using thread calls
    /// and then passing data or commands back and forth to it
    pub fn subcall<A>(&self, format: SerializationFormat, req: A) -> CallBuilder
    where
        A: Serialize,
    {
        super::subcall(
            self.handle.clone(),
            format,
            req,
        )
    }

    pub fn handle(&self) -> CallSmartHandle {
        self.handle.clone()
    }
}

impl<T> Deref for DetachedCall<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.result
    }
}

#[derive(Debug, Clone)]
#[must_use = "this `Call` only does something when you consume it"]
pub struct CallJoin<T>
where
    T: de::DeserializeOwned,
{
    call: Call,
    scope: CallSmartHandle,
    _marker1: PhantomData<T>,
}

impl<T> CallJoin<T>
where
    T: de::DeserializeOwned,
{
    fn new(call: Call, scope: CallSmartHandle) -> CallJoin<T> {
        CallJoin {
            call,
            scope,
            _marker1: PhantomData,
        }
    }

    /// Waits for the call to complete and returns the response from
    /// the server
    #[cfg(feature = "rt")]
    pub fn wait(self) -> Result<T, BusError> {
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
    pub fn try_wait(&mut self) -> Result<Option<T>, BusError>
    where
        T: de::DeserializeOwned,
    {
        let response = {
            let mut state = self.call.state.lock().unwrap();
            state.result.take()
        };

        match response {
            Some(Ok(res)) => {
                let res = res.format.deserialize(res.data);
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
    type Output = Result<T, BusError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let response = {
            let mut state = self.call.state.lock().unwrap();
            state.result.take()
        };

        match response {
            Some(Ok(res)) => {
                let res = res.format.deserialize(res.data);
                match res {
                    Ok(data) => Poll::Ready(Ok(data)),
                    Err(err) => Poll::Ready(Err(err)),
                }
            }
            Some(Err(err)) => Poll::Ready(Err(err)),
            None => {
                crate::engine::BusEngine::subscribe(&self.scope.cid(), cx);
                Poll::Pending
            }
        }
    }
}
