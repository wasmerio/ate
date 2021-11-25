mod data;
mod call;
mod handle;
mod error;
mod recv;
mod reply;
pub(crate) mod syscall;

use serde::*;
use std::any::type_name;
use std::borrow::Cow;

pub use error::*;
pub use data::*;
pub use handle::*;
pub use call::*;
pub use recv::*;
pub use reply::*;

use crate::reqwest::IntoUrlSealed;

pub fn call<T>(wapm: Cow<'static, str>, request: T) -> CallBuilder
where T: Serialize, 
{
    let topic = type_name::<T>();
    let call = crate::engine::BusEngine::call(None, wapm, topic.into());
    
    let req = match bincode::serialize(&request) {
        Ok(req) => Data::Success(req),
        Err(_err) => {
            Data::Error(CallError::SerializationFailed)
        }
    };

    CallBuilder::new(call, req)
}

pub fn call_recursive<T>(handle: CallHandle, wapm: Cow<'static, str>, request: T) -> CallBuilder
where T: Serialize, 
{
    let topic = type_name::<T>();
    let call = crate::engine::BusEngine::call(Some(handle), wapm, topic.into());
    
    let req = match bincode::serialize(&request) {
        Ok(req) => Data::Success(req),
        Err(_err) => {
            Data::Error(CallError::SerializationFailed)
        }
    };

    CallBuilder::new(call, req)
}

pub fn recv<RES, REQ>() -> Recv<RES, REQ>
where REQ: de::DeserializeOwned + Send + Sync + 'static,
      RES: Serialize + Send + Sync + 'static
{
    let topic = type_name::<REQ>();
    let recv = crate::engine::BusEngine::recv();
    let handle = recv.handle;

    syscall::recv(handle, topic.as_str());

    recv
}

pub fn recv_recursive<RES, REQ>(handle: CallHandle)
where REQ: de::DeserializeOwned + Send + Sync + 'static,
      RES: Serialize + Send + Sync + 'static
{
    let topic = type_name::<REQ>();
    syscall::recv(handle, topic.as_str());
}

pub(self) fn reply<RES>(handle: CallHandle, response: RES)
where RES: Serialize
{
    match bincode::serialize(&response) {
        Ok(res) => {
            syscall::reply(handle, &res[..]);
        }
        Err(_err) => {
            syscall::error(handle, CallError::SerializationFailed as i32)
        }
    }
}

pub(self) fn drop(handle: CallHandle)
{
    syscall::drop(handle);
}

pub fn thread_id() -> u32
{
    syscall::thread_id()
}