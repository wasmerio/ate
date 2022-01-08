use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;
use wasm_bus::abi::SerializationFormat;
use tokio::sync::mpsc;

use super::*;

#[derive(Clone)]
pub struct WasmBusCallback {
    callback_tx: mpsc::Sender<WasmBusThreadResult>,
    handle: CallHandle,
}

impl WasmBusCallback {
    pub fn new(thread: &WasmBusThread, handle: CallHandle) -> Result<WasmBusCallback, CallError> {
        Ok(WasmBusCallback {
            callback_tx: thread.callback_tx.clone(),
            handle,
        })
    }

    pub async fn feed<T>(&self, format: SerializationFormat, data: T)
    where
        T: Serialize,
    {
        self.feed_bytes_or_error(super::encode_response(format, &data)).await;
    }

    pub async fn feed_or_error<T>(&self, format: SerializationFormat, data: Result<T, CallError>)
    where
        T: Serialize,
    {
        let data = match data.map(|a| super::encode_response(format, &a)) {
            Ok(a) => a,
            Err(err) => Err(err),
        };
        self.feed_bytes_or_error(data).await;
    }

    pub async fn feed_bytes(&self, data: Vec<u8>) {
        trace!(
            "wasm-bus::call-reply (handle={}, response={} bytes)",
            self.handle.id,
            data.len()
        );

        let _ = self.callback_tx.send(WasmBusThreadResult::Response {
            handle: self.handle,
            data,
        }).await;
    }

    pub async fn feed_bytes_or_error(&self, data: Result<Vec<u8>, CallError>) {
        match data {
            Ok(a) => self.feed_bytes(a).await,
            Err(err) => self.error(err).await,
        };
    }

    pub async fn error(&self, err: CallError) {
        trace!(
            "wasm-bus::call-reply (handle={}, error={})",
            self.handle.id,
            err
        );
        let _ = self.callback_tx.send(WasmBusThreadResult::Failed {
            handle: self.handle,
            err,
        }).await;
    }
}
