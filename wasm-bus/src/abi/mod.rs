mod call;
mod data;
mod error;
mod handle;
mod recv;
mod reply;
pub(crate) mod syscall;

use serde::*;
use std::any::type_name;
use std::borrow::Cow;

pub use call::*;
pub use data::*;
pub use error::*;
pub use handle::*;
pub use recv::*;
pub use reply::*;

pub fn call<T>(wapm: Cow<'static, str>, request: T) -> CallBuilder
where
    T: Serialize,
{
    call_internal(None, wapm, request)
}

pub(crate) fn call_internal<T>(
    parent: Option<CallHandle>,
    wapm: Cow<'static, str>,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    let topic = type_name::<T>();
    let call = crate::engine::BusEngine::call(parent, wapm, topic.into());

    let req = match bincode::serialize(&request) {
        Ok(req) => Data::Success(req),
        Err(_err) => Data::Error(CallError::SerializationFailed),
    };

    CallBuilder::new(call, req)
}

pub fn recv<RES, REQ, F>(callback: F) -> Recv
where
    REQ: de::DeserializeOwned + Send + Sync + 'static,
    RES: Serialize + Send + Sync + 'static,
    F: Fn(REQ) -> Result<RES, CallError>,
    F: Send + Sync + 'static,
{
    recv_internal::<RES, REQ, F>(None, callback)
}

pub(crate) fn recv_internal<RES, REQ, F>(parent: Option<CallHandle>, callback: F) -> Recv
where
    REQ: de::DeserializeOwned + Send + Sync + 'static,
    RES: Serialize + Send + Sync + 'static,
    F: FnMut(REQ) -> Result<RES, CallError>,
    F: Send + 'static,
{
    let topic = type_name::<REQ>();
    let recv = crate::engine::BusEngine::recv(callback);
    let handle = recv.handle;

    syscall::recv(parent, handle, topic);

    recv
}

pub(self) fn reply<RES>(handle: CallHandle, response: RES)
where
    RES: Serialize,
{
    match bincode::serialize(&response) {
        Ok(res) => {
            syscall::reply(handle, &res[..]);
        }
        Err(_err) => syscall::error(handle, CallError::SerializationFailed as i32),
    }
}

pub(self) fn drop(handle: CallHandle) {
    syscall::drop(handle);
}

pub fn thread_id() -> u32 {
    syscall::thread_id()
}
