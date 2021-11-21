use super::*;

mod raw {
    #[link(wasm_import_module = "wasm-bus")]
    extern "C" {
        // Common calls
        pub(crate) fn drop(handle: u32);
        pub(crate) fn rand() -> u32;

        // Server calls
        pub(crate) fn recv(handle: u32, topic: i32, topic_len: i32);
        pub(crate) fn error(handle: u32, error: i32);
        pub(crate) fn reply(handle: u32, response: i32, response_len: i32);

        // Client calls
        pub(crate) fn call(handle: u32, wapm: i32, wapm_len: i32, topic: i32, topic_len: i32, request: i32, request_len: i32);
        pub(crate) fn call_recursive(parent: u32, handle: u32, topic: i32, topic_len: i32, request: i32, request_len: i32);
        pub(crate) fn recv_recursive(parent: u32, handle: u32, topic: i32, topic_len: i32);
        pub(crate) fn yield_and_wait(timeout_ms: u32);
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

// Server calls
pub fn recv(handle: CallHandle, topic: &str) {
    unsafe {
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        raw::recv(handle.id, topic as i32, topic_len as i32)
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

// Client calls
pub fn call(handle: CallHandle, wapm: &str, topic: &str, request: &[u8]) {
    unsafe {
        let wapm_len = wapm.len();
        let wapm = wapm.as_ptr();
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        let request_len = request.len();
        let request = request.as_ptr();
        raw::call(handle.id, wapm as i32, wapm_len as i32, topic as i32, topic_len as i32, request as i32, request_len as i32);
    }
}
pub fn call_recursive(parent: CallHandle, handle: CallHandle, topic: &str, request: &[u8]) {
    unsafe {
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        let request_len = request.len();
        let request = request.as_ptr();
        raw::call_recursive(parent.id, handle.id, topic as i32, topic_len as i32, request as i32, request_len as i32);
    }
}
pub fn recv_recursive(parent: CallHandle, handle: CallHandle, topic: &str) {
    unsafe {
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        raw::recv_recursive(parent.id, handle.id, topic as i32, topic_len as i32);
    }
}
pub fn yield_and_wait(timeout_ms: u32) {
    unsafe {
        raw::yield_and_wait(timeout_ms);
    }
}