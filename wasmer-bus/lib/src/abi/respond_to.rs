use derivative::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_bus_types::SerializationFormat;

use crate::abi::BusError;
use crate::abi::CallHandle;

pub enum RespondAction
{
    Response(Vec<u8>),
    Fault(BusError),
    Detach
}

pub enum RespondActionTyped<T>
{
    Response(T),
    Fault(BusError),
    Detach
}

type CallbackHandler = Arc<
    dyn Fn(CallHandle, Vec<u8>) -> Pin<Box<dyn Future<Output = RespondAction> + Send>>
        + Send
        + Sync,
>;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct RespondToService {
    pub(crate) format: SerializationFormat,
    #[derivative(Debug = "ignore")]
    pub(crate) callbacks: Arc<Mutex<HashMap<CallHandle, CallbackHandler>>>,
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
                    -> Pin<Box<dyn Future<Output = RespondAction> + Send>>
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
                return;
            }
        };

        let mut leak = false;
        let res = callback.as_ref()(handle, request);
        match res.await {
            RespondAction::Response(a) => {
                crate::abi::syscall::call_reply(handle, &a[..], format);
            }
            RespondAction::Fault(err) => {
                crate::abi::syscall::call_fault(handle, err);
            }
            RespondAction::Detach => {
                leak = true;
            }
        }
        if leak == false {
            crate::engine::BusEngine::close(&handle, "request was processed (by listener)");
        }
    }
}
