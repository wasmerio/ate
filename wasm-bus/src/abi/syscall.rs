#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use super::*;

mod raw {
    use super::*;

    #[no_mangle]
    pub(super) extern "C" fn wasm_bus_free(buf_ptr: u32, buf_len: u32) {
        trace!("wasm_bus_free (buf={} bytes)", buf_len);
        unsafe {
            let data = Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize);
            std::mem::drop(data);
        }
    }

    #[no_mangle]
    pub(super) extern "C" fn wasm_bus_malloc(len: u32) -> u32 {
        trace!("wasm_bus_malloc (len={})", len);
        let mut buf = Vec::with_capacity(len as usize);
        let ptr: *mut u8 = buf.as_mut_ptr();
        std::mem::forget(buf);
        return ptr as u32;
    }

    // Invoked when the call has finished
    #[no_mangle]
    pub(super) extern "C" fn wasm_bus_data(handle: u32, topic: i32, topic_len: i32, response: u32, response_len: u32) {
        trace!("wasm_bus_data (handle={}, response={} bytes)", handle, response_len);
        unsafe {
            let topic = String::from_raw_parts(topic as *mut u8, topic_len as usize, topic_len as usize);
            let response = Vec::from_raw_parts(response as *mut u8, response_len as usize, response_len as usize);
            crate::engine::BusEngine::put(handle.into(), topic, response);
        }
    }

    // Invoked when the call has failed
    #[no_mangle]
    pub(super) extern "C" fn wasm_bus_error(handle: u32, error: u32) {
        trace!("wasm_bus_err (handle={}, error={})", handle, error);
        crate::engine::BusEngine::error(handle.into(), error.into());
    }

    #[link(wasm_import_module = "wasm-bus")]
    extern "C" {
        pub(crate) fn drop(handle: u32);
        pub(crate) fn rand() -> u32;
        pub(crate) fn yield_and_wait(timeout_ms: u32);
        
        pub(crate) fn error(handle: u32, error: i32);
        pub(crate) fn reply(handle: u32, response: i32, response_len: i32);

        pub(crate) fn call(handle: u32, wapm: i32, wapm_len: i32, topic: i32, topic_len: i32, request: i32, request_len: i32) -> u32;
        pub(crate) fn recv(handle: u32, topic: i32, topic_len: i32);

        pub(crate) fn thread_id() -> u32;
    }
}

pub fn drop(handle: CallHandle) {
    unsafe {
        raw::drop(handle.id)
    }
}
pub fn rand() -> u32 {
    unsafe {
        raw::rand()
    }
}

pub fn error(handle: CallHandle, error: i32) {
    unsafe {
        raw::error(handle.id, error);
    }
}

pub fn reply(handle: CallHandle, response: &[u8]) {
    unsafe {
        let response_len = response.len();
        let response = response.as_ptr();
        raw::reply(handle.id, response as i32, response_len as i32);
    }
}

pub fn call(handle: CallHandle, wapm: &str, topic: &str, request: &[u8]) {
    let ret = unsafe {
        let wapm_len = wapm.len();
        let wapm = wapm.as_ptr();
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        let request_len = request.len();
        let request = request.as_ptr();
        raw::call(handle.id, wapm as i32, wapm_len as i32, topic as i32, topic_len as i32, request as i32, request_len as i32)
    };

    if CallError::Success as u32 != ret {
        raw::wasm_bus_error(handle.id, ret);
    }
}

pub fn recv(handle: CallHandle, topic: &str) {
    unsafe {
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        raw::recv(handle.id, topic as i32, topic_len as i32)
    }
}

pub fn yield_and_wait(timeout_ms: u32) {
    unsafe {
        raw::yield_and_wait(timeout_ms);
    }
}

pub fn thread_id() -> u32 {
    unsafe {
        raw::thread_id()
    }
}