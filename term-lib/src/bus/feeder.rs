use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;
use wasm_bus::abi::SerializationFormat;
use tokio::sync::mpsc;

use crate::api::System;
use crate::api::SystemAbiExt;
use super::*;

#[derive(Clone)]
pub struct WasmBusCallback {
    system: System,
    callback_tx: mpsc::Sender<WasmBusThreadResult>,
    handle: CallHandle,
}

impl WasmBusCallback {
    pub fn new(thread: &WasmBusThread, handle: CallHandle) -> Result<WasmBusCallback, CallError> {
        Ok(WasmBusCallback {
            system: thread.system,
            callback_tx: thread.callback_tx.clone(),
            handle,
        })
    }

    pub fn feed<T>(&self, format: SerializationFormat, data: T)
    where
        T: Serialize,
    {
        self.feed_bytes_or_error(super::encode_response(format, &data));
    }

    pub fn feed_or_error<T>(&self, format: SerializationFormat, data: Result<T, CallError>)
    where
        T: Serialize,
    {
        let data = match data.map(|a| super::encode_response(format, &a)) {
            Ok(a) => a,
            Err(err) => Err(err),
        };
        self.feed_bytes_or_error(data);
    }

    pub fn feed_bytes(&self, data: Vec<u8>) {
        trace!(
            "wasm-bus::call-reply (handle={}, response={} bytes)",
            self.handle.id,
            data.len()
        );

        self.system.fork_send(&self.callback_tx, WasmBusThreadResult::Response {
            handle: self.handle,
            data,
        });
    }

    pub fn feed_bytes_or_error(&self, data: Result<Vec<u8>, CallError>) {
        match data {
            Ok(a) => self.feed_bytes(a),
            Err(err) => self.error(err),
        };
    }

    pub fn error(&self, err: CallError) {
        trace!(
            "wasm-bus::call-reply (handle={}, error={})",
            self.handle.id,
            err
        );
        self.system.fork_send(&self.callback_tx, WasmBusThreadResult::Failed {
            handle: self.handle,
            err,
        });
    }
}
