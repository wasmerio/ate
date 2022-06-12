use derivative::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_types::SerializationFormat;

use crate::abi::BusError;
use crate::abi::CallHandle;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ListenService {
    pub(crate) format: SerializationFormat,
    #[derivative(Debug = "ignore")]
    pub(crate) callback: Arc<
        dyn Fn(
                CallHandle,
                Vec<u8>,
            ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, BusError>> + Send>>
            + Send
            + Sync,
    >,
    pub(crate) persistent: bool,
}

impl ListenService {
    pub fn new(
        format: SerializationFormat,
        callback: Arc<
            dyn Fn(
                    CallHandle,
                    Vec<u8>,
                )
                    -> Pin<Box<dyn Future<Output = Result<Vec<u8>, BusError>> + Send>>
                + Send
                + Sync,
        >,
        persistent: bool,
    ) -> ListenService {
        ListenService {
            format,
            callback,
            persistent,
        }
    }

    pub async fn process(&self, handle: CallHandle, request: Vec<u8>, format: SerializationFormat) {
        let callback = Arc::clone(&self.callback);
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
            crate::engine::BusEngine::close(&handle, "request was processed (by listener)");
        }
    }
}
