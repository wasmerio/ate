use serde::*;
use std::marker::PhantomData;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::sync::Arc;
use std::sync::Mutex;
use derivative::*;

use super::*;

pub struct RecvState<T>
{
    pub(crate) response: Option<Result<T, CallError>>,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[must_use = "you must 'wait' or 'await' to receive any calls from other modules"]
pub struct Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    pub(crate) handle: CallHandle,
    pub(crate) topic: Cow<'static, str>,
    #[derivative(Debug="ignore")]
    pub(crate) state: Arc<Mutex<RecvState<REQ>>>,
    pub(crate) _marker1: PhantomData<REQ>,
    pub(crate) _marker2: PhantomData<RES>,
}

impl<RES, REQ> CallOps
for Recv<RES, REQ>
where REQ: de::DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync
{
    fn data(&self, topic: String, data: Vec<u8>) {
        let res = bincode::deserialize::<REQ>(data.as_ref())
                .map_err(|_err| CallError::DeserializationFailed);

        let mut state = self.state.lock().unwrap();
        if topic == self.topic {
            state.response.replace(res);
        }
    }

    fn error(&self, error: CallError) {
        let mut state = self.state.lock().unwrap();
        state.response.replace(Err(error));
    }
}

impl<RES, REQ> Drop
for Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    fn drop(&mut self) {
        super::drop(self.handle);
    }
}

impl<RES, REQ> Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    #[allow(dead_code)]
    pub(crate) fn new(handle: CallHandle) -> Recv<RES, REQ> {
        let topic = std::any::type_name::<REQ>();
        Recv {
            handle,
            topic: topic.into(),
            state: Arc::new(Mutex::new(RecvState {
                response: None
            })),
            _marker1: PhantomData,
            _marker2: PhantomData
        }
    }

    pub fn id(&self) -> u32 {
        self.handle.id
    }

    pub fn wait(self) -> Result<Reply<RES, REQ>, CallError> {
        crate::backend::block_on::block_on(self)
    }

    pub fn try_wait(&mut self) -> Result<Option<Reply<RES, REQ>>, CallError> {
        let mut state = self.state.lock().unwrap();
        if let Some(result) = state.response.take() {
            match result {
                Ok(request) => {
                    Ok(Some(Reply::new(self.handle, request)))
                },
                Err(err) => {
                    Err(err)
                }
            }
        } else {
            Ok(None)
        }
    }
}

impl<RES, REQ> Future
for Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    type Output = Result<Reply<RES, REQ>, CallError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
    {
        let request = {            
            let mut state = self.state.lock().unwrap();
            state.response.take()
        };

        if let Some(request) = request {
            match request {
                Ok(request) => {
                    Poll::Ready(Ok(Reply::new(self.handle, request)))
                },
                Err(err) => {
                    Poll::Ready(Err(err))
                }
            }
        } else {
            crate::engine::BusEngine::subscribe(&self.handle, cx);
            Poll::Pending
        }
    }
}