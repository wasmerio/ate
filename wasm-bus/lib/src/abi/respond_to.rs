use derivative::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_types::SerializationFormat;

use crate::abi::BusError;
use crate::abi::CallHandle;

type CallbackHandler = Arc<
    dyn Fn(CallHandle, Vec<u8>) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, BusError>> + Send>>
        + Send
        + Sync,
>;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct RespondToService {
    pub(crate) format: SerializationFormat,
    #[derivative(Debug = "ignore")]
    pub(crate) callbacks: Arc<Mutex<HashMap<CallHandle, CallbackHandler>>>,
    pub(crate) persistent: bool,
}

impl RespondToService {
    pub fn new(format: SerializationFormat, persistent: bool) -> RespondToService {
        RespondToService {
            format,
            callbacks: Default::default(),
            persistent,
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
                    -> Pin<Box<dyn Future<Output = Result<Vec<u8>, BusError>> + Send>>
                + Send
                + Sync,
        >,
    ) {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.insert(handle, callback);
    }

    pub fn remove(&self, handle: &CallHandle) -> Option<CallbackHandler> {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.remove(handle)
    }

    pub async fn process(&self, callback_handle: CallHandle, handle: CallHandle, request: Vec<u8>, format: SerializationFormat) {
        let callback = {
            let callbacks = self.callbacks.lock().unwrap();
            if let Some(callback) = callbacks.get(&callback_handle) {
                Arc::clone(callback)
            } else {
                crate::abi::syscall::call_fault(handle, BusError::InvalidHandle);
                crate::engine::BusEngine::close(&handle, "invalid callback handle");
                return;
            }
        };

        let res = callback.as_ref()(handle, request);
        match res.await {
            Ok(a) => {
                crate::abi::syscall::call_reply(handle, &a[..], format);
            }
            Err(err) => {
                crate::abi::syscall::call_fault(handle, err);
            }
        }
        if self.persistent == false {
            crate::engine::BusEngine::close(&handle, "request was processed (by respond_to)");
        }
    }
}
