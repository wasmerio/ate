use super::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

mod raw {
    use super::*;

    #[no_mangle]
    pub extern "C" fn wasm_bus_free(buf_ptr: u32, buf_len: u32) {
        trace!("wasm_bus_free (buf={} bytes)", buf_len);
        unsafe {
            let data = Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize);
            std::mem::drop(data);
        }
    }

    #[no_mangle]
    pub extern "C" fn wasm_bus_malloc(len: u32) -> u32 {
        trace!("wasm_bus_malloc (len={})", len);
        let mut buf = Vec::with_capacity(len as usize);
        let ptr: *mut u8 = buf.as_mut_ptr();
        std::mem::forget(buf);
        return ptr as u32;
    }

    // Invoked when the call has finished
    #[no_mangle]
    pub extern "C" fn wasm_bus_data(handle: u32, data: u32, data_len: u32) {
        trace!(
            "wasm_bus_data (handle={}, response={} bytes)",
            handle,
            data_len
        );
        unsafe {
            let response =
                Vec::from_raw_parts(data as *mut u8, data_len as usize, data_len as usize);

            match crate::engine::BusEngine::put(handle.into(), response) {
                Some(Ok(response)) => {
                    let _response_len = response.len();
                    let _response = response.as_ptr();
                    //reply(handle, response as i32, response_len as i32);
                }
                Some(Err(_err)) => {
                    //fault(handle, err as i32);
                }
                None => {}
            };
        }
    }

    // Invoked when the call has failed
    #[no_mangle]
    pub extern "C" fn wasm_bus_error(handle: u32, error: u32) {
        trace!("wasm_bus_err (handle={}, error={})", handle, error);
        crate::engine::BusEngine::error(handle.into(), error.into());
    }

    #[link(wasm_import_module = "wasm-bus")]
    extern "C" {
        pub(crate) fn drop(handle: u32);
        pub(crate) fn rand() -> u32;
        pub(crate) fn yield_and_wait(timeout_ms: u32);

        pub(crate) fn fault(handle: u32, error: i32);
        pub(crate) fn reply(handle: u32, response: i32, response_len: i32);

        pub(crate) fn call(
            parent: u32,
            handle: u32,
            wapm: i32,
            wapm_len: i32,
            topic: i32,
            topic_len: i32,
            request: i32,
            request_len: i32,
        ) -> u32;
        pub(crate) fn recv(parent: u32, handle: u32, topic: i32, topic_len: i32);

        pub(crate) fn thread_id() -> u32;
    }
}

pub fn drop(handle: CallHandle) {
    unsafe { raw::drop(handle.id) }
}
pub fn rand() -> u32 {
    unsafe { raw::rand() }
}

pub fn error(handle: CallHandle, error: i32) {
    unsafe {
        raw::fault(handle.id, error);
    }
}

pub fn reply(handle: CallHandle, response: &[u8]) {
    unsafe {
        let response_len = response.len();
        let response = response.as_ptr();
        raw::reply(handle.id, response as i32, response_len as i32);
    }
}

pub fn call(
    parent: Option<CallHandle>,
    handle: CallHandle,
    wapm: &str,
    topic: &str,
    request: &[u8],
) {
    let ret = unsafe {
        let parent = parent.map(|a| a.id).unwrap_or_else(|| u32::MAX);
        let wapm_len = wapm.len();
        let wapm = wapm.as_ptr();
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        let request_len = request.len();
        let request = request.as_ptr();
        raw::call(
            parent,
            handle.id,
            wapm as i32,
            wapm_len as i32,
            topic as i32,
            topic_len as i32,
            request as i32,
            request_len as i32,
        )
    };

    if CallError::Success as u32 != ret {
        raw::wasm_bus_error(handle.id, ret);
    }
}

pub fn recv(parent: Option<CallHandle>, handle: CallHandle, topic: &str) {
    unsafe {
        let parent = parent.map(|a| a.id).unwrap_or_else(|| u32::MAX);
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        raw::recv(parent, handle.id, topic as i32, topic_len as i32)
    }
}

pub fn yield_and_wait(timeout_ms: u32) {
    unsafe {
        raw::yield_and_wait(timeout_ms);
    }
}

pub fn thread_id() -> u32 {
    unsafe { raw::thread_id() }
}
