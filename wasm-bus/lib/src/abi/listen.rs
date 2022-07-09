use derivative::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_types::SerializationFormat;

use crate::abi::BusError;
use super::CallHandle;

pub enum ListenAction
{
    Response(Vec<u8>),
    Fault(BusError),
    Detach
}

pub enum ListenActionTyped<T>
{
    Response(T),
    Fault(BusError),
    Detach
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ListenService {
    pub(crate) format: SerializationFormat,
    #[derivative(Debug = "ignore")]
    pub(crate) callback: Arc<
        dyn Fn(
                CallHandle,
                Vec<u8>,
            ) -> Pin<Box<dyn Future<Output = ListenAction> + Send>>
            + Send
            + Sync,
    >,
}

impl ListenService {
    pub fn new(
        format: SerializationFormat,
        callback: Arc<
            dyn Fn(
                    CallHandle,
                    Vec<u8>,
                )
                    -> Pin<Box<dyn Future<Output = ListenAction> + Send>>
                + Send
                + Sync,
        >,
    ) -> ListenService {
        ListenService {
            format,
            callback,
        }
    }

    pub async fn process(&self, handle: CallHandle, request: Vec<u8>, format: SerializationFormat) {
        let callback = Arc::clone(&self.callback);
        let res = callback.as_ref()(handle, request);

        let mut leak = false;
        match res.await {
            ListenAction::Response(a) => {
                crate::abi::syscall::call_reply(handle, &a[..], format);
            }
            ListenAction::Fault(err) => {
                crate::abi::syscall::call_fault(handle, err);
            }
            ListenAction::Detach => {
                leak = true;
            }
        }
        if leak == false {
            crate::engine::BusEngine::close(&handle, "request was processed (by listener)");
        }
    }
}
