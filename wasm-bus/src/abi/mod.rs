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
mod session;
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
pub use session::*;

pub use wasm_bus_types::*;

pub fn call<T>(
    wapm: Cow<'static, str>,
    format: SerializationFormat,
    session: Option<String>,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    call_internal(None, wapm, format, session, request)
}

pub(crate) fn call_internal<T>(
    parent: Option<CallHandle>,
    wapm: Cow<'static, str>,
    format: SerializationFormat,
    session: Option<String>,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    let topic = type_name::<T>();
    let call = crate::engine::BusEngine::call(parent, wapm, topic.into(), format, session);

    let req = match format {
        SerializationFormat::Bincode => match bincode::serialize(&request) {
            Ok(req) => Data::Success(req),
            Err(_err) => Data::Error(CallError::SerializationFailed),
        },
        SerializationFormat::Json => match serde_json::to_vec(&request) {
            Ok(req) => Data::Success(req),
            Err(_err) => Data::Error(CallError::SerializationFailed),
        },
    };

    CallBuilder::new(call, req)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn callback_internal<RES, REQ, F>(
    parent: CallHandle,
    format: SerializationFormat,
    callback: F,
) -> Finish
where
    REQ: de::DeserializeOwned + Send + Sync + 'static,
    RES: Serialize + Send + Sync + 'static,
    F: FnMut(REQ) -> Result<RES, CallError>,
    F: Send + 'static,
{
    let topic = type_name::<REQ>();
    let recv = crate::engine::BusEngine::callback(format, callback);
    let handle = recv.handle;

    syscall::callback(parent, handle, topic);
    return recv;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn callback_internal<RES, REQ, F>(
    _parent: CallHandle,
    _format: SerializationFormat,
    _callback: F,
) -> Finish
where
    REQ: de::DeserializeOwned + Send + Sync + 'static,
    RES: Serialize + Send + Sync + 'static,
    F: FnMut(REQ) -> Result<RES, CallError>,
    F: Send + 'static,
{
    panic!("recv not supported on this platform");
}

#[cfg(all(target_arch = "wasm32"))]
pub(self) fn reply<RES>(handle: CallHandle, format: SerializationFormat, response: RES)
where
    RES: Serialize,
{
    match format {
        SerializationFormat::Bincode => match bincode::serialize(&response) {
            Ok(res) => {
                syscall::reply(handle, &res[..]);
            }
            Err(_err) => syscall::fault(handle, CallError::SerializationFailed as u32),
        },
        SerializationFormat::Json => match serde_json::to_vec(&response) {
            Ok(res) => {
                syscall::reply(handle, &res[..]);
            }
            Err(_err) => syscall::fault(handle, CallError::SerializationFailed as u32),
        },
    };
}

#[cfg(not(target_arch = "wasm32"))]
pub(self) fn reply<RES>(_handle: CallHandle, _format: SerializationFormat, _response: RES)
where
    RES: Serialize,
{
    panic!("reply not supported on this platform");
}

#[cfg(all(target_arch = "wasm32"))]
pub fn reply_callback<RES>(handle: CallHandle, format: SerializationFormat, response: RES)
where
    RES: Serialize,
{
    let topic = type_name::<RES>();
    match format {
        SerializationFormat::Bincode => match bincode::serialize(&response) {
            Ok(res) => {
                syscall::reply(handle, &res[..]);
            }
            Err(_err) => syscall::fault(handle, CallError::SerializationFailed as u32),
        },
        SerializationFormat::Json => match serde_json::to_vec(&response) {
            Ok(res) => {
                syscall::reply_callback(handle, topic, &res[..]);
            }
            Err(_err) => syscall::fault(handle, CallError::SerializationFailed as u32),
        },
    };
}

#[cfg(not(target_arch = "wasm32"))]
pub fn reply_callback<RES>(_handle: CallHandle, _format: SerializationFormat, _response: RES)
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
