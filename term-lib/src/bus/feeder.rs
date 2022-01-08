use serde::*;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;
use wasm_bus::abi::SerializationFormat;

use super::*;
use crate::wasmer::Memory;
use crate::wasmer::NativeFunc;

#[derive(Clone)]
pub struct WasmBusCallback {
    memory: Memory,
    native_finish: NativeFunc<(u32, u32, u32), ()>,
    native_malloc: NativeFunc<u32, u32>,
    native_error: NativeFunc<(u32, u32), ()>,
    waker: Arc<ThreadWaker>,
    handle: CallHandle,
}

impl WasmBusCallback {
    pub fn new(thread: &WasmBusThread, handle: CallHandle) -> WasmBusCallback {
        let memory = thread.memory().clone();
        let native_data = thread.wasm_bus_finish_ref();
        let native_malloc = thread.wasm_bus_malloc_ref();
        let native_error = thread.wasm_bus_error_ref();

        WasmBusCallback {
            memory,
            native_finish: native_data.unwrap().clone(),
            native_malloc: native_malloc.unwrap().clone(),
            native_error: native_error.unwrap().clone(),
            waker: thread.waker.clone(),
            handle,
        }
    }

    pub(crate) fn waker(&self) -> Arc<ThreadWaker> {
        self.waker.clone()
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

        let buf_len = data.len() as u32;
        let buf = self.native_malloc.call(buf_len).unwrap();

        self.memory
            .uint8view_with_byte_offset_and_length(buf, buf_len)
            .copy_from(&data[..]);

        self.native_finish
            .call(self.handle.id, buf, buf_len)
            .unwrap();
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
        self.native_error.call(self.handle.id, err.into()).unwrap();
    }
}
