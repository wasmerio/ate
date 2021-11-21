use std::ops::*;
use std::marker::PhantomData;
use serde::*;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use derivative::*;
use std::sync::Arc;
use std::sync::Mutex;

use super::*;
use crate::engine;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[must_use = "you must 'wait' or 'await' to actually send this call to other modules"]
pub struct Call<T>
where T: de::DeserializeOwned
{
    handle: CallHandle,
    #[derivative(Debug="ignore")]
    callbacks: Arc<Mutex<Vec<Pin<Box<dyn Future<Output=()> + Send + 'static>>>>>,
    _marker: PhantomData<T>,
}

impl<T> Drop
for Call<T>
where T: de::DeserializeOwned
{
    fn drop(&mut self) {
        super::drop(self.handle);
    }
}

impl<T> Call<T>
where T: de::DeserializeOwned
{
    pub fn id(&self) -> u32 {
        self.handle.id
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallbackLifetime {
    KeepGoing,
    Drop
}

pub trait IntoCallbackReturn
{
    fn into_lifetime(self) -> Pin<Box<dyn Future<Output=CallbackLifetime> + Send + 'static>>;
}

impl IntoCallbackReturn
for CallbackLifetime
{
    fn into_lifetime(self) -> Pin<Box<dyn Future<Output=CallbackLifetime> + Send + 'static>> {
        Box::pin(async move {
            self
        })
    }
}

impl<T> IntoCallbackReturn
for T
where T: Future<Output=CallbackLifetime> + Send + 'static
{
    fn into_lifetime(self) -> Pin<Box<dyn Future<Output=CallbackLifetime> + Send + 'static>> {
        Box::pin(self)
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
#[must_use = "you must 'invoke' the builder for it to actually call anything"]
pub struct CallBuilder<T>
where T: de::DeserializeOwned
{
    handle: CallHandle,
    #[derivative(Debug="ignore")]
    callbacks: Vec<Pin<Box<dyn Future<Output=()> + Send + 'static>>>,
    _marker: PhantomData<T>,
}

impl<T> CallBuilder<T>
where T: de::DeserializeOwned
{
    pub fn new(handle: CallHandle) -> CallBuilder<T> {
        CallBuilder {
            handle,
            callbacks: Vec::new(),
            _marker: PhantomData
        }
    }
}

impl<T> CallBuilder<T>
where T: de::DeserializeOwned
{
    /// Upon receiving a particular message from the service that is
    /// invoked this callback will take some action
    /// (this function handles both synchonrous and asynchronout
    ///  callbacks - just return a Callbacklifetime using either)
    pub fn with_callback<C, F, Fut>(mut self, mut callback: F) -> Self
    where C: Serialize + de::DeserializeOwned + Send,
          F: FnMut(C) -> Fut,
          F: Send + 'static,
          Fut: IntoCallbackReturn,
    {
        let handle = self.handle;
        let callbacks = &mut self.callbacks;
        callbacks.push(Box::pin(
            async move {
                loop {
                    let recv = super::recv_recursive::<(), C>(handle);
                    if let Ok(reply) = recv.await {
                        let lifetime = callback(reply.take()).into_lifetime();
                        if CallbackLifetime::Drop == lifetime.await {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        ));
        self
    }

    /// Invokes the call with the specified callbacks
    pub fn invoke(self) -> Call<T>
    {
        Call {
            handle: self.handle,
            callbacks: Arc::new(Mutex::new(self.callbacks)),
            _marker: PhantomData
        }
    }
}

impl<T> Call<T>
where T: de::DeserializeOwned
{
    /// Creates another call relative to this call
    /// This can be useful for creating contextual objects using thread calls
    /// and then passing data or commands back and forth to it
    pub fn call<RES, REQ>(&self, req: REQ) -> CallBuilder<RES>
    where REQ: Serialize,
          RES: de::DeserializeOwned
    {
        super::call_recursive(self.handle, req)
    }

    /// Creates a recv operation on the client side
    /// This can be useful for creating contextual objects using thread calls
    /// and then passing data or commands back and forth to it
    pub fn recv<RES, REQ>(&self) -> Recv<RES, REQ>
    where REQ: de::DeserializeOwned,
        RES: Serialize
    {
        super::recv_recursive(self.handle)
    }

    /// Waits for the call to complete and returns the response from
    /// the server
    pub fn wait(self) -> Result<T, CallError> {
        crate::backend::block_on::block_on(self)
    }

    /// Tries to get the result of the call to the server but will not
    /// block the execution
    pub fn try_wait(&mut self) -> Result<Option<T>, CallError> {
        match engine::poll(&self.handle, None) {
            Some(Data::Success(res)) => {
                bincode::deserialize::<T>(res.as_ref())
                    .map(|a| Some(a))
                    .map_err(|_err| CallError::SerializationFailed)
            },
            Some(Data::Error(err)) => {
                Err(err)
            }
            None => Ok(None)
        }
    }
}

impl<T> Future
for Call<T>
where T: de::DeserializeOwned
{
    type Output = Result<T, CallError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
    {
        {
            let mut callbacks = self.callbacks.lock().unwrap();
            for index in (0..callbacks.len()).rev() {
                let callback = Pin::new(&mut callbacks[index]);
                match callback.poll(cx) {
                    Poll::Ready(_) => {
                        callbacks.remove(index);
                    }
                    Poll::Pending => {
                        return Poll::Pending;
                    }
                }
            }
        }

        let res = match engine::poll(&self.handle, Some(cx)) {
            Some(Data::Success(a)) => a,
            Some(Data::Error(err)) => {
                self.callbacks.lock().unwrap().clear();
                return Poll::Ready(Err(err));
            }
            None => {
                return Poll::Pending;
            }
        };
        let res = bincode::deserialize::<T>(res.as_ref())
            .map_err(|_err| CallError::DeserializationFailed)?;
        self.callbacks.lock().unwrap().clear();
        Poll::Ready(Ok(res))
    }
}