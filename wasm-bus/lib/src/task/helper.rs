use serde::*;
use std::any::type_name;
use std::future::Future;
use std::ops::Deref;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::abi::BusError;
use crate::abi::CallHandle;
use crate::abi::CallSmartHandle;
use crate::abi::SerializationFormat;
use crate::engine::BusEngine;
use crate::rt::RuntimeBlockingGuard;
use crate::rt::RUNTIME;

#[cfg(target_os = "wasi")]
pub fn block_on<F>(task: F) -> F::Output
where
    F: Future,
{
    RUNTIME.block_on(task)
}

#[cfg(not(target_os = "wasi"))]
#[cfg(feature = "sys")]
pub fn block_on<F>(task: F) -> F::Output
where
    F: Future,
{
    tokio::task::block_in_place(move || {
        tokio::runtime::Handle::current().block_on(async move {
            task.await
        })
    })
}

#[cfg(not(target_os = "wasi"))]
#[cfg(not(feature = "sys"))]
pub fn block_on<F>(_task: F) -> F::Output
where
    F: Future,
{
    unimplemented!();
}

pub fn blocking_guard() -> RuntimeBlockingGuard {
    RuntimeBlockingGuard::new(RUNTIME.deref())
}

#[cfg(target_os = "wasi")]
pub fn spawn<F>(task: F)
where
    F: Future + Send + 'static,
{
    RUNTIME.spawn(task)
}

#[cfg(not(target_os = "wasi"))]
#[cfg(feature = "sys")]
pub fn spawn<F>(task: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::task::spawn(task)
}

#[cfg(not(target_os = "wasi"))]
#[cfg(not(feature = "sys"))]
pub fn spawn<F>(_task: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    unimplemented!();
}

#[cfg(target_os = "wasi")]
pub fn wake() {
    RUNTIME.wake();
}

#[cfg(not(target_os = "wasi"))]
pub fn wake() {
}

pub fn serve() {
    RUNTIME.serve();
}

pub fn work_it() -> usize {
    RUNTIME.tick()
}

pub fn listen<RES, REQ, F, Fut>(format: SerializationFormat, callback: F, persistent: bool)
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
    F: Fn(CallHandle, REQ) -> Fut,
    F: Send + Sync + 'static,
    Fut: Future<Output = Result<RES, BusError>> + Send + 'static,
{
    let topic = type_name::<REQ>();
    BusEngine::listen_internal(
        format,
        topic.to_string(),
        move |handle, req| {
            let req = match format.deserialize(req) {
                Ok(a) => a,
                Err(err) => {
                    debug!("failed to deserialize the request object (type={}, format={}) - {}", type_name::<REQ>(), format, err);
                    return Err(BusError::DeserializationFailed);
                }
            };

            let res = callback(handle, req);

            Ok(async move {
                let res = res.await?;
                let res = format.serialize(res)
                    .map_err(|err| {
                        debug!(
                            "failed to serialize the response object (type={}, format={}) - {}",
                            type_name::<RES>(),
                            format,
                            err
                        );
                        BusError::SerializationFailed
                    })?;
                Ok(res)
            })
        },
        persistent,
    );
}

pub fn respond_to<RES, REQ, F, Fut>(
    parent: CallSmartHandle,
    format: SerializationFormat,
    callback: F,
    persistent: bool,
) where
    REQ: de::DeserializeOwned,
    RES: Serialize,
    F: Fn(CallHandle, REQ) -> Fut,
    F: Send + Sync + 'static,
    Fut: Future<Output = Result<RES, BusError>> + Send + 'static,
{
    let topic = type_name::<REQ>();
    BusEngine::respond_to_internal(
        format,
        topic.to_string(),
        parent,
        move |handle, req| {
            let req = match format.deserialize(req) {
                Ok(a) => a,
                Err(err) => {
                    debug!("failed to deserialize the request object (type={}, format={}) - {}", type_name::<REQ>(), format, err);
                    return Err(BusError::DeserializationFailed);
                }
            };

            let res = callback(handle, req);

            Ok(async move {
                let res = res.await?;
                let res = format.serialize(res) .map_err(|err| {
                    debug!(
                        "failed to serialize the response object (type={}, format={}) - {}",
                        type_name::<RES>(),
                        format,
                        err
                    );
                    BusError::SerializationFailed
                })?;
                Ok(res)
            })
        },
        persistent,
    );
}
