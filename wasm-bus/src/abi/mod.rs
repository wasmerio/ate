mod call;
mod data;
mod error;
mod finish;
mod handle;
#[cfg(feature = "rt")]
mod listen;
mod reply;
#[cfg(feature = "rt")]
mod respond_to;
#[cfg(feature = "syscalls")]
pub(crate) mod syscall;
#[cfg(not(feature = "syscalls"))]
pub(crate) mod unsupported;

#[allow(unused_imports)]
#[cfg(not(feature = "syscalls"))]
pub(crate) use unsupported as syscall;

use serde::*;
use std::any::type_name;
use std::borrow::Cow;

pub use call::*;
pub use data::*;
pub use error::*;
pub use finish::*;
pub use handle::*;
#[cfg(feature = "rt")]
pub use listen::*;
pub use reply::*;
#[cfg(feature = "rt")]
pub use respond_to::*;

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

#[cfg(target_arch = "wasm32")]
pub(crate) fn callback_internal<RES, REQ, F>(parent: CallHandle, callback: F) -> Finish
where
    REQ: de::DeserializeOwned + Send + Sync + 'static,
    RES: Serialize + Send + Sync + 'static,
    F: FnMut(REQ) -> Result<RES, CallError>,
    F: Send + 'static,
{
    let topic = type_name::<REQ>();
    let recv = crate::engine::BusEngine::callback(callback);
    let handle = recv.handle;

    syscall::callback(parent, handle, topic);
    return recv;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn callback_internal<RES, REQ, F>(_parent: CallHandle, _callback: F) -> Finish
where
    REQ: de::DeserializeOwned + Send + Sync + 'static,
    RES: Serialize + Send + Sync + 'static,
    F: FnMut(REQ) -> Result<RES, CallError>,
    F: Send + 'static,
{
    panic!("recv not supported on this platform");
}

#[cfg(all(target_arch = "wasm32"))]
pub(self) fn reply<RES>(handle: CallHandle, response: RES)
where
    RES: Serialize,
{
    match bincode::serialize(&response) {
        Ok(res) => {
            syscall::reply(handle, &res[..]);
        }
        Err(_err) => syscall::fault(handle, CallError::SerializationFailed as u32),
    };
}

#[cfg(not(target_arch = "wasm32"))]
pub(self) fn reply<RES>(_handle: CallHandle, _response: RES)
where
    RES: Serialize,
{
    panic!("reply not supported on this platform");
}

#[cfg(target_arch = "wasm32")]
pub(self) fn drop(handle: CallHandle) {
    syscall::drop(handle);
}

#[cfg(not(target_arch = "wasm32"))]
pub(self) fn drop(_handle: CallHandle) {
    panic!("drop handle not supported on this platform");
}

#[cfg(target_arch = "wasm32")]
pub fn thread_id() -> u32 {
    syscall::thread_id()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn thread_id() -> u32 {
    panic!("thread_id not supported on this platform");
}
