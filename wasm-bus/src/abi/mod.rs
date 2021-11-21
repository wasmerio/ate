mod data;
mod call;
mod handle;
mod error;
mod recv;
mod reply;
pub(crate) mod syscall;

use serde::*;
use std::any::type_name;

pub use error::*;
pub use data::*;
pub use handle::*;
pub use call::*;
pub use recv::*;
pub use reply::*;

use crate::reqwest::IntoUrlSealed;

pub fn call<RES, REQ>(wapm: &str, request: REQ) -> CallBuilder<RES>
where REQ: Serialize, 
      RES: de::DeserializeOwned
{
    let topic = type_name::<REQ>();
    let handle = crate::engine::begin();
    
    match bincode::serialize(&request) {
        Ok(req) => {
            syscall::call(handle, wapm.as_str(), topic, &req[..]);
        }
        Err(_err) => {
            crate::engine::finish(handle, Data::Error(CallError::SerializationFailed));
        }
    }

    CallBuilder::new(handle)
}

pub(self) fn call_recursive<RES, REQ>(parent: CallHandle, request: REQ) -> CallBuilder<RES>
where REQ: Serialize, 
      RES: de::DeserializeOwned
{
    let handle = crate::engine::begin();
    
    match bincode::serialize(&request) {
        Ok(req) => {
            syscall::call_recursive(parent, handle, &req[..]);
        }
        Err(_err) => {
            crate::engine::finish(handle, Data::Error(CallError::SerializationFailed));
        }
    }

    CallBuilder::new(handle)
}

pub fn recv<RES, REQ>() -> Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    let topic = type_name::<REQ>();
    let handle = crate::engine::begin();
    syscall::recv(handle, topic.as_str());

    Recv::new(handle)
}

pub fn recv_recursive<RES, REQ>(parent: CallHandle) -> Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    let topic = type_name::<REQ>();
    let handle = crate::engine::begin();
    syscall::recv_recursive(parent, handle, topic);
    
    Recv::new(handle)
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