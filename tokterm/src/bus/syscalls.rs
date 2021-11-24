#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::sync::Arc;
use wasm_bus::abi::CallError;
use wasmer::Array;
use wasmer::WasmPtr;

use super::thread::WasmBusThread;

pub fn wasm_bus_drop(_thread: &WasmBusThread, handle: u32) {
    info!("wasm-bus::drop (handle={})", handle);
}

pub fn wasm_bus_rand(_thread: &WasmBusThread) -> u32 {
    info!("wasm-bus::rand");
    fastrand::u32(..)
}

pub fn wasm_bus_tick(_thread: &WasmBusThread) {
    //info!("wasm-bus::tick");
}

pub fn wasm_bus_recv(thread: &WasmBusThread, handle: u32, topic: WasmPtr<u8, Array>, topic_len: u32) {
    let topic = unsafe { topic.get_utf8_str(thread.memory(), topic_len).unwrap() };
    info!("wasm-bus::recv (handle={}, topic={})", handle, topic);
}

pub fn wasm_bus_recv_recursive(
    thread: &WasmBusThread,
    parent: u32,
    handle: u32,
    topic: WasmPtr<u8, Array>,
    topic_len: u32,
) {
    let topic = unsafe { topic.get_utf8_str(thread.memory(), topic_len).unwrap() };
    info!(
        "wasm-bus::recv_recursive (parent={}, handle={}, topic={})",
        parent, handle, topic
    );
}

pub fn wasm_bus_error(_thread: &WasmBusThread, handle: u32, error: i32) {
    info!("wasm-bus::error (handle={}, error={})", handle, error);
}

pub fn wasm_bus_reply(
    thread: &WasmBusThread,
    handle: u32,
    response: WasmPtr<u8, Array>,
    response_len: u32,
) {
    info!(
        "wasm-bus::reply (handle={}, response={} bytes)",
        handle, response_len
    );

    // Grab the data we are sending back
    let _response = thread.memory()
            .uint8view()
            .subarray(response.offset(), response_len)
            .to_vec();
}

pub fn wasm_bus_call(
    thread: &WasmBusThread,
    handle: u32,
    wapm: WasmPtr<u8, Array>,
    wapm_len: u32,
    topic: WasmPtr<u8, Array>,
    topic_len: u32,
    request: WasmPtr<u8, Array>,
    request_len: u32,
) -> u32
{
    let wapm = unsafe { wapm.get_utf8_str(thread.memory(), wapm_len).unwrap() };
    let topic = unsafe { topic.get_utf8_str(thread.memory(), topic_len).unwrap() };
    info!(
        "wasm-bus::call (handle={}, wapm={}, topic={}, request={} bytes)",
        handle, wapm, topic, request_len
    );
    
    let request = thread.memory()
            .uint8view()
            .subarray(request.offset(), request_len)
            .to_vec();

    // Start the sub-process and invoke the call
    let invoke = {
        let invoke = thread.factory.start(wapm.as_ref(), topic.as_ref());
        let mut invocations = thread.invocations.write().unwrap();
        invocations.insert(handle, Arc::clone(&invoke));
        invoke
    };

    // Grab references to the ABI that will be used
    let error_callback = thread.wasm_bus_error_ref();
    let malloc_callback = thread.wasm_bus_malloc_ref();
    let data_callback = thread.wasm_bus_data_ref();
    if error_callback.is_none() || malloc_callback.is_none() || data_callback.is_none() {
        info!("wasm-bus::call-reply (incorrect abi)");
        return CallError::IncorrectAbi.into();
    }
    let error_callback = error_callback.unwrap().clone();
    let malloc_callback = malloc_callback.unwrap().clone();
    let data_callback = data_callback.unwrap().clone();

    // Invoke the send operation
    let topic_copy = topic.to_string();
    let thread = thread.clone();
    invoke.send(request, Box::new(move |response| {
        match response {
            Ok(data) => {
                info!("wasm-bus::call-reply (handle={}, response={} bytes)", handle, data.len());

                let topic_len = topic_copy.len() as u32;
                let topic = malloc_callback.call(topic_len).unwrap();

                thread.memory()
                    .uint8view()
                    .subarray(topic.offset(), topic_len)
                    .copy_from(&topic_copy.as_bytes()[..]);

                let buf_len = data.len() as u32;
                let buf = malloc_callback.call(buf_len).unwrap();

                thread.memory()
                    .uint8view()
                    .subarray(buf.offset(), buf_len)
                    .copy_from(&data[..]);

                data_callback.call(handle, topic, topic_len, buf, buf_len).unwrap();
            },
            Err(err) => {
                info!("wasm-bus::call-reply (handle={}, error={})", handle, err);
                error_callback.call(handle, err.into()).unwrap();
            }
        }
    }));

    CallError::Success.into()
}

pub fn wasm_bus_call_recursive(
    thread: &WasmBusThread,
    parent: u32,
    handle: u32,
    topic: WasmPtr<u8, Array>,
    topic_len: u32,
    request: WasmPtr<u8, Array>,
    request_len: u32,
) -> u32
{
    let topic = unsafe { topic.get_utf8_str(thread.memory(), topic_len).unwrap() };
    info!(
        "wasm-bus::call_recursive (parent={}, handle={}, topic={}, request={} bytes)",
        parent, handle, topic, request_len
    );

    let _request = thread.memory()
            .uint8view()
            .subarray(request.offset(), request_len)
            .to_vec();

    //CallError::Unknown.into()
    CallError::Success.into()
}

pub fn wasm_bus_yield_and_wait(_thread: &WasmBusThread, timeout_ms: u32) {
    info!("wasm-bus::yield_and_wait (timeout={} ms)", timeout_ms);
}

pub fn wasm_bus_thread_id(thread: &WasmBusThread) -> u32 {
    info!("wasm-bus::thread_id (id={})", thread.thread_id);
    thread.thread_id
}