mod buffer;
mod string;
mod data;
mod call;
mod handle;
mod error;
mod recv;
mod reply;
pub(crate) mod syscall;

use serde::*;
use std::any::type_name;

pub(self) use buffer::*;
pub(self) use string::*;
pub use error::*;
pub use data::*;
pub(crate) use handle::*;
pub use call::*;
pub use recv::*;
pub use reply::*;

pub fn call<RES, REQ>(wapm: &str, request: REQ) -> CallBuilder<RES>
where REQ: Serialize, 
      RES: de::DeserializeOwned
{
    let topic = type_name::<REQ>();
    let handle = crate::engine::begin();
    
    match bincode::serialize(&request) {
        Ok(req) => {
            let req: Buffer = req.into();
            unsafe {
                syscall::call(handle, wapm.into(), topic.into(), req);
            }
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
            let req: Buffer = req.into();
            unsafe {
                syscall::call_recursive(parent, handle, req);
            }
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
    unsafe {
        syscall::recv(handle, topic.into())
    };

    Recv::new(handle)
}

pub fn recv_recursive<RES, REQ>(parent: CallHandle) -> Recv<RES, REQ>
where REQ: de::DeserializeOwned,
      RES: Serialize
{
    let topic = type_name::<REQ>();
    let handle = crate::engine::begin();
    unsafe {
        syscall::recv_recursive(parent, handle, topic.into())
    };

    Recv::new(handle)
}

pub(self) fn reply<RES>(handle: CallHandle, response: RES)
where RES: Serialize
{
    match bincode::serialize(&response) {
        Ok(res) => {
            let res: Buffer = res.into();
            unsafe {
                syscall::reply(handle, res);
            }
        }
        Err(_err) => {
            unsafe {
                syscall::error(handle, CallError::SerializationFailed)
            }
        }
    }
}

pub(self) fn drop(handle: CallHandle)
{
    unsafe {
        syscall::drop(handle);
    }
}