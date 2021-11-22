#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer::Array;
use wasmer::WasmPtr;

use super::env::WasmBusEnv;

// Common calls
pub fn wasm_bus_drop(env: &WasmBusEnv, handle: u32) {
    info!("wasm-bus::drop (handle={})", handle);
}

pub fn wasm_bus_rand(env: &WasmBusEnv) -> u32 {
    info!("wasm-bus::rand");
    fastrand::u32(..)
}

// Server calls
pub fn wasm_bus_recv(env: &WasmBusEnv, handle: u32, topic: WasmPtr<u8, Array>, topic_len: u32) {
    let topic = unsafe { topic.get_utf8_str(env.memory(), topic_len).unwrap() };
    info!("wasm-bus::drop (handle={}, topic={})", handle, topic);
}
pub fn wasm_bus_error(env: &WasmBusEnv, handle: u32, error: i32) {
    info!("wasm-bus::error (handle={}, error={})", handle, error);
}
pub fn wasm_bus_reply(env: &WasmBusEnv, handle: u32, response: WasmPtr<u8, Array>, response_len: u32) {
    info!("wasm-bus::reply (handle={}, response={} bytes)", handle, response_len);
}

// Client calls
pub fn wasm_bus_call(env: &WasmBusEnv, handle: u32, wapm: WasmPtr<u8, Array>, wapm_len: u32, topic: WasmPtr<u8, Array>, topic_len: u32, request: WasmPtr<u8, Array>, request_len: u32) {
    let wapm = unsafe { wapm.get_utf8_str(env.memory(), wapm_len).unwrap() };
    let topic = unsafe { topic.get_utf8_str(env.memory(), topic_len).unwrap() };
    info!("wasm-bus::call (handle={}, wapm={}, topic={}, request={} bytes)", handle, wapm, topic, request_len);
}
pub fn wasm_bus_call_recursive(env: &WasmBusEnv, parent: u32, handle: u32, topic: WasmPtr<u8, Array>, topic_len: u32, request: WasmPtr<u8, Array>, request_len: u32) {
    let topic = unsafe { topic.get_utf8_str(env.memory(), topic_len).unwrap() };
    info!("wasm-bus::call_recursive (parent={}, handle={}, topic={}, request={} bytes)", parent, handle, topic, request_len);
}
pub fn wasm_bus_recv_recursive(env: &WasmBusEnv, parent: u32, handle: u32, topic: WasmPtr<u8, Array>, topic_len: u32) {
    let topic = unsafe { topic.get_utf8_str(env.memory(), topic_len).unwrap() };
    info!("wasm-bus::recv_recursive (parent={}, handle={}, topic={})", parent, handle, topic);
}
pub fn wasm_bus_yield_and_wait(env: &WasmBusEnv, timeout_ms: u32)  {
    info!("wasm-bus::yield_and_wait (timeout={} ms)", timeout_ms);
}