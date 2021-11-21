use serde::*;
use std::marker::PhantomData;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use super::*;
use crate::engine;

#[derive(Debug, Clone)]
#[must_use = "you must 'wait' or 'await' to receive any calls from other modules"]
pub struct Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    handle: CallHandle,
    _marker1: PhantomData<REQ>,
    _marker2: PhantomData<RES>,
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
    pub(crate) fn new(handle: CallHandle) -> Recv<RES, REQ> {
        Recv {
            handle,
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
        match engine::poll(&self.handle, None) {
            Some(Data::Success(request)) => {
                bincode::deserialize::<REQ>(request.as_ref())
                    .map(|request| {
                        Some(Reply::new(self.handle, request))
                    })
                    .map_err(|_err| CallError::DeserializationFailed)
            },
            Some(Data::Error(err)) => {
                Err(err)
            }
            None => Ok(None)
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
        let request = match engine::poll(&self.handle, Some(cx)) {
            Some(Data::Success(a)) => a,
            Some(Data::Error(err)) => {
                return Poll::Ready(Err(err));
            }
            None => {
                return Poll::Pending;
            }
        };
        let request = bincode::deserialize::<REQ>(request.as_ref())
            .map_err(|_err| CallError::DeserializationFailed)?;
        Poll::Ready(Ok(Reply::new(self.handle, request)))
    }
}