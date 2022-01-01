use derivative::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_types::SerializationFormat;

use crate::abi::CallError;
use crate::abi::CallHandle;
use crate::task::spawn;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct RespondToService {
    pub(crate) format: SerializationFormat,
    #[derivative(Debug = "ignore")]
    pub(crate) callbacks: Arc<
        Mutex<
            HashMap<
                CallHandle,
                Arc<
                    dyn Fn(
                            CallHandle,
                            Vec<u8>,
                        )
                            -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CallError>> + Send>>
                        + Send
                        + Sync,
                >,
            >,
        >,
    >,
}

impl RespondToService {
    pub fn new(format: SerializationFormat) -> RespondToService {
        RespondToService {
            format,
            callbacks: Default::default(),
        }
    }

    pub fn add(
        &self,
        handle: CallHandle,
        callback: Arc<
            dyn Fn(
                    CallHandle,
                    Vec<u8>,
                )
                    -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CallError>> + Send>>
                + Send
                + Sync,
        >,
    ) {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.insert(handle, callback);
    }

    pub fn remove(&self, handle: &CallHandle) {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.remove(handle);
    }

    pub fn process(&self, callback_handle: CallHandle, handle: CallHandle, request: Vec<u8>) {
        let format = self.format.clone();
        let callback = {
            let callbacks = self.callbacks.lock().unwrap();
            if let Some(callback) = callbacks.get(&callback_handle) {
                Arc::clone(callback)
            } else {
                spawn(async move {
                    let err: u32 = CallError::InvalidHandle.into();
                    crate::abi::syscall::fault(handle, err as u32);
                    crate::engine::BusEngine::remove(&handle);
                });
                return;
            }
        };

        spawn(async move {
            let res = callback.as_ref()(handle, request);
            match res.await {
                Ok(a) => {
                    crate::abi::syscall::reply(handle, &a[..]);
                }
                Err(CallError::Fork) => {
                    // The idea behind this is so that when a client request is made
                    // that starts an interface that the function can yield from the
                    // method without closing down the handle (the client will have
                    // to manually close the handle themselves)
                    let res = match format {
                        SerializationFormat::Bincode => bincode::serialize(&()).unwrap(),
                        SerializationFormat::Json => serde_json::to_vec(&()).unwrap(),
                    };
                    crate::abi::syscall::reply(handle, &res[..]);
                    return;
                }
                Err(err) => {
                    let err: u32 = err.into();
                    crate::abi::syscall::fault(handle, err as u32);
                }
            }
            crate::engine::BusEngine::remove(&handle);
        });
    }
}
