use super::*;
use std::sync::atomic::AtomicBool;

#[no_mangle]
extern "C" fn wasm_bus_free(buf: Buffer) {
    buf.manual_drop();
}

#[no_mangle]
extern "C" fn wasm_bus_malloc(size: usize) -> Buffer {
    vec![0u8; size].into()
}

// Invoked when the call has finished
#[no_mangle]
extern "C" fn wasm_bus_data(handle: CallHandle, response: Data) {
    crate::engine::finish(handle, response);
}

#[link(wasm_import_module = "wasm-bus")]
extern "C" {
    // Common calls
    pub(crate) fn drop(handle: CallHandle);

    // Server calls
    pub(crate) fn recv(handle: CallHandle, topic: BString);
    pub(crate) fn error(handle: CallHandle, error: CallError);
    pub(crate) fn reply(handle: CallHandle, response: Buffer);

    // Client calls
    pub(crate) fn call(handle: CallHandle, wapm: BString, topic: BString, request: Buffer);
    pub(crate) fn call_recursive(parent: CallHandle, handle: CallHandle, request: Buffer);
    pub(crate) fn recv_recursive(parent: CallHandle, handle: CallHandle, topic: BString);
    pub(crate) fn yield_and_wait(waker: *const AtomicBool, timeout_ms: u32);
}