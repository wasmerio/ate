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
//#[cfg(target_os = "wasi")]
pub(crate) mod syscall;
#[cfg(not(target_os = "wasi"))]
pub(crate) mod unsupported;

//#[allow(unused_imports)]
//#[cfg(not(target_os = "wasi"))]
//pub(crate) use unsupported as syscall;

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
    instance: Option<CallInstance>,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    call_ext(None, wapm, format, instance, request)
}

pub fn call_ext<T>(
    parent: Option<CallHandle>,
    wapm: Cow<'static, str>,
    format: SerializationFormat,
    instance: Option<CallInstance>,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    let topic = type_name::<T>();
    let call = crate::engine::BusEngine::call(parent, wapm, topic.into(), format, instance);

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
    
    crate::engine::BusEngine::add_callback(parent.clone(), handle.clone());
    syscall::callback(parent, handle, topic);
    return recv;
}

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

pub fn reply_callback<RES>(handle: CallHandle, format: SerializationFormat, response: RES)
where
    RES: Serialize,
{
    let topic = type_name::<RES>();
    match format {
        SerializationFormat::Bincode => match bincode::serialize(&response) {
            Ok(res) => {
                syscall::reply_callback(handle, topic, &res[..]);
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

pub(self) fn drop(handle: CallHandle) {
    syscall::drop(handle);
}

pub fn thread_id() -> u32 {
    syscall::thread_id()
}
