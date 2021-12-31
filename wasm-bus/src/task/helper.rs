use serde::*;
use std::any::type_name;
use std::future::Future;

use crate::abi::CallError;
use crate::abi::CallHandle;
use crate::abi::SerializationFormat;
use crate::engine::BusEngine;
use crate::rt::RUNTIME;

pub fn block_on<F>(task: F) -> F::Output
where
    F: Future,
{
    RUNTIME.block_on(task)
}

pub fn spawn<F>(task: F)
where
    F: Future + Send + 'static,
{
    RUNTIME.spawn(task)
}

pub fn wake() {
    RUNTIME.wake();
}

pub fn serve() {
    RUNTIME.serve();
}

pub fn listen<RES, REQ, F, Fut>(format: SerializationFormat, callback: F)
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
    F: Fn(CallHandle, REQ) -> Fut,
    F: Send + Sync + 'static,
    Fut: Future<Output = Result<RES, CallError>> + Send + 'static,
{
    let topic = type_name::<REQ>();
    BusEngine::listen_internal(topic.to_string(), move |handle, req| {
        let req = match format {
            SerializationFormat::Bincode => match bincode::deserialize(&req[..]) {
                Ok(a) => a,
                Err(_) => {
                    return Err(CallError::DeserializationFailed);
                }
            },
            SerializationFormat::Json => match serde_json::from_slice(&req[..]) {
                Ok(a) => a,
                Err(_) => {
                    return Err(CallError::DeserializationFailed);
                }
            },
        };

        let res = callback(handle, req);

        Ok(async move {
            let res = res.await?;
            let res = match format {
                SerializationFormat::Bincode => {
                    bincode::serialize(&res).map_err(|_| CallError::SerializationFailed)?
                }
                SerializationFormat::Json => {
                    serde_json::to_vec(&res).map_err(|_| CallError::SerializationFailed)?
                }
            };
            Ok(res)
        })
    });
}

pub fn respond_to<RES, REQ, F, Fut>(parent: CallHandle, format: SerializationFormat, callback: F)
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
    F: Fn(CallHandle, REQ) -> Fut,
    F: Send + Sync + 'static,
    Fut: Future<Output = Result<RES, CallError>> + Send + 'static,
{
    let topic = type_name::<REQ>();
    BusEngine::respond_to_internal(topic.to_string(), parent, move |handle, req| {
        let req = match format {
            SerializationFormat::Bincode => match bincode::deserialize(&req[..]) {
                Ok(a) => a,
                Err(_) => {
                    return Err(CallError::DeserializationFailed);
                }
            },
            SerializationFormat::Json => match serde_json::from_slice(&req[..]) {
                Ok(a) => a,
                Err(_) => {
                    return Err(CallError::DeserializationFailed);
                }
            },
        };

        let res = callback(handle, req);

        Ok(async move {
            let res = res.await?;
            let res = match format {
                SerializationFormat::Bincode => {
                    bincode::serialize(&res).map_err(|_| CallError::SerializationFailed)?
                }
                SerializationFormat::Json => {
                    serde_json::to_vec(&res).map_err(|_| CallError::SerializationFailed)?
                }
            };
            Ok(res)
        })
    });
}
