use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;

use super::*;
use crate::wasmer::Array;
use crate::wasmer::Memory;
use crate::wasmer::NativeFunc;
use crate::wasmer::WasmPtr;

#[derive(Clone)]
pub struct WasmBusFeeder {
    memory: Memory,
    native_data: NativeFunc<(u32, WasmPtr<u8, Array>, u32), ()>,
    native_malloc: NativeFunc<u32, WasmPtr<u8, Array>>,
    native_error: NativeFunc<(u32, u32), ()>,
    handle: u32,
}

impl WasmBusFeeder {
    pub fn new(thread: &WasmBusThread, handle: u32) -> Result<WasmBusFeeder, CallError> {
        let memory = thread.memory().clone();
        let native_data = thread.wasm_bus_data_ref();
        let native_malloc = thread.wasm_bus_malloc_ref();
        let native_error = thread.wasm_bus_error_ref();

        if native_data.is_none() || native_malloc.is_none() || native_error.is_none() {
            debug!("wasm-bus::feeder (incorrect abi)");
            return Err(CallError::IncorrectAbi.into());
        }

        Ok(WasmBusFeeder {
            memory,
            native_data: native_data.unwrap().clone(),
            native_malloc: native_malloc.unwrap().clone(),
            native_error: native_error.unwrap().clone(),
            handle,
        })
    }

    pub fn feed<T>(&self, data: T)
    where
        T: Serialize,
    {
        self.feed_bytes_or_error(super::encode_response(&data));
    }

    pub fn feed_bytes(&self, data: Vec<u8>) {
        trace!(
            "wasm-bus::call-reply (handle={}, response={} bytes)",
            self.handle,
            data.len()
        );

        let buf_len = data.len() as u32;
        let buf = self.native_malloc.call(buf_len).unwrap();

        self.memory
            .uint8view_with_byte_offset_and_length(buf.offset(), buf_len)
            .copy_from(&data[..]);

        self.native_data.call(self.handle, buf, buf_len).unwrap();
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
            self.handle,
            err
        );
        self.native_error.call(self.handle, err.into()).unwrap();
    }
}
