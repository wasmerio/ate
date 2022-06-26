mod call;
mod data;
mod finish;
mod handle;
#[cfg(feature = "rt")]
mod listen;
mod reply;
#[cfg(feature = "rt")]
mod respond_to;
mod session;
#[cfg(target_os = "wasi")]
pub(crate) mod syscall;
#[cfg(not(target_os = "wasi"))]
pub(crate) mod unsupported;

#[cfg(not(target_os = "wasi"))]
pub(crate) use unsupported as syscall;

use std::any::type_name;
use std::borrow::Cow;

pub use call::*;
pub use data::*;
pub use finish::*;
pub use handle::*;
#[cfg(feature = "rt")]
pub use listen::*;
pub use reply::*;
#[cfg(feature = "rt")]
pub use respond_to::*;
use serde::Serialize;
pub use session::*;

pub use wasm_bus_types::*;

pub fn call<T>(
    ctx: CallContext,
    format: SerializationFormat,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    match ctx {
        CallContext::NewBusCall { wapm, instance } => {
            call_new(wapm, instance, format, request)
        },
        CallContext::SubCall { parent } => {
            subcall(parent, format, request)
        }
    }
}

pub fn call_new<T>(
    wapm: Cow<'static, str>,
    instance: Option<CallInstance>,
    format: SerializationFormat,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    let topic = type_name::<T>();
    let topic_hash = crate::engine::hash_topic(&topic.into());
    let call = Call::new_call(wapm, topic_hash, instance);

    let req = match format.serialize(request) {
        Ok(req) => Data::Prepared(req),
        Err(err) => Data::Error(err),
    };

    CallBuilder::new(call, req, format)
}

pub fn subcall<T>(
    parent: CallSmartHandle,
    format: SerializationFormat,
    request: T,
) -> CallBuilder
where
    T: Serialize,
{
    let topic = type_name::<T>();
    let topic_hash = crate::engine::hash_topic(&topic.into());
    let call = Call::new_subcall(parent, topic_hash);

    let req = match format.serialize(request) {
        Ok(req) => Data::Prepared(req),
        Err(err) => Data::Error(err),
    };

    CallBuilder::new(call, req, format)
}

pub(self) fn reply<RES>(handle: CallHandle, format: SerializationFormat, response: RES)
where
    RES: Serialize,
{
    match format.serialize(response) {
        Ok(res) => {
            syscall::call_reply(handle, &res[..], format);
        }
        Err(_err) => syscall::call_fault(handle, BusError::SerializationFailed),
    }
}
