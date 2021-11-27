use std::future::Future;
use std::task::Context;
use std::task::Poll;
use std::time::Instant;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;
use wasmer::Array;
use wasmer::WasmPtr;

use super::thread::WasmBusThread;
use super::*;

pub(crate) mod raw {
    use super::*;
    pub fn wasm_bus_drop(thread: &WasmBusThread, handle: u32) {
        unsafe { super::wasm_bus_drop(thread, handle) }
    }
    pub fn wasm_bus_rand(thread: &WasmBusThread) -> u32 {
        unsafe { super::wasm_bus_rand(thread) }
    }
    pub fn wasm_bus_tick(thread: &WasmBusThread) {
        unsafe { super::wasm_bus_tick(thread) }
    }
    pub fn wasm_bus_recv(
        thread: &WasmBusThread,
        parent: u32,
        handle: u32,
        topic: WasmPtr<u8, Array>,
        topic_len: u32,
    ) {
        unsafe { super::wasm_bus_recv(thread, parent, handle, topic, topic_len) }
    }
    pub fn wasm_bus_fault(thread: &WasmBusThread, handle: u32, error: i32) {
        unsafe { super::wasm_bus_fault(thread, handle, error) }
    }
    pub fn wasm_bus_reply(
        thread: &WasmBusThread,
        handle: u32,
        response: WasmPtr<u8, Array>,
        response_len: u32,
    ) {
        unsafe { super::wasm_bus_reply(thread, handle, response, response_len) }
    }
    pub fn wasm_bus_call(
        thread: &WasmBusThread,
        parent: u32,
        handle: u32,
        wapm: WasmPtr<u8, Array>,
        wapm_len: u32,
        topic: WasmPtr<u8, Array>,
        topic_len: u32,
        request: WasmPtr<u8, Array>,
        request_len: u32,
    ) -> u32 {
        unsafe {
            super::wasm_bus_call(
                thread,
                parent,
                handle,
                wapm,
                wapm_len,
                topic,
                topic_len,
                request,
                request_len,
            )
        }
    }
    pub fn wasm_bus_yield_and_wait(thread: &WasmBusThread, timeout_ms: u32) {
        unsafe { super::wasm_bus_yield_and_wait(thread, timeout_ms) }
    }
    pub fn wasm_bus_thread_id(thread: &WasmBusThread) -> u32 {
        unsafe { super::wasm_bus_thread_id(thread) }
    }
}

unsafe fn wasm_bus_drop(thread: &WasmBusThread, handle: u32) {
    thread.inner.unwrap().factory.close(CallHandle::from(handle));
}

unsafe fn wasm_bus_rand(_thread: &WasmBusThread) -> u32 {
    fastrand::u32(..)
}

unsafe fn wasm_bus_tick(thread: &WasmBusThread) {
    // Take the invocations out of the idle list and process them
    // (we need to do this outside of the thread local lock as
    //  otherwise the re-entrance will panic the system)
    let invocations = {
        let mut inner = thread.inner.unwrap();
        inner.invocations.drain().collect::<Vec<_>>()
    };

    // Run all the invocations and build a carry over list
    let waker = dummy_waker::dummy_waker();
    let mut cx = Context::from_waker(&waker);
    let mut carry_over = Vec::new();
    for (key, mut invocation) in invocations {
        let pinned_invocation = invocation.as_mut();
        if let Poll::Pending = pinned_invocation.poll(&mut cx) {
            carry_over.push((key, invocation));
        }
    }

    // If there are any carry overs then readd them
    if carry_over.is_empty() == false {
        let mut inner = thread.inner.unwrap();
        for (key, invoke) in carry_over {
            inner.invocations.insert(key, invoke);
        }
    }
}

unsafe fn wasm_bus_recv(
    thread: &WasmBusThread,
    parent: u32,
    handle: u32,
    topic: WasmPtr<u8, Array>,
    topic_len: u32,
) {
    let topic = topic.get_utf8_str(thread.memory(), topic_len).unwrap();
    debug!(
        "wasm-bus::recv (parent={}, handle={}, topic={})",
        parent, handle, topic
    );
}

unsafe fn wasm_bus_fault(_thread: &WasmBusThread, handle: u32, error: i32) {
    debug!("wasm-bus::error (handle={}, error={})", handle, error);
}

unsafe fn wasm_bus_reply(
    thread: &WasmBusThread,
    handle: u32,
    response: WasmPtr<u8, Array>,
    response_len: u32,
) {
    debug!(
        "wasm-bus::reply (handle={}, response={} bytes)",
        handle, response_len
    );

    // Grab the data we are sending back
    let _response = thread
        .memory()
        .uint8view_with_byte_offset_and_length(response.offset(), response_len)
        .to_vec();
}

unsafe fn wasm_bus_call(
    thread: &WasmBusThread,
    parent: u32,
    handle: u32,
    wapm: WasmPtr<u8, Array>,
    wapm_len: u32,
    topic: WasmPtr<u8, Array>,
    topic_len: u32,
    request: WasmPtr<u8, Array>,
    request_len: u32,
) -> u32 {
    let parent = if parent != u32::MAX {
        Some(CallHandle::from(parent))
    } else {
        None
    };
    let wapm = wapm.get_utf8_str(thread.memory(), wapm_len).unwrap();
    let topic = topic.get_utf8_str(thread.memory(), topic_len).unwrap();
    if let Some(parent) = parent {
        let parent: u32 = parent.into();
        debug!(
            "wasm-bus::call (parent={}, handle={}, wapm={}, topic={}, request={} bytes)",
            parent, handle, wapm, topic, request_len
        );
    } else {
        debug!(
            "wasm-bus::call (handle={}, wapm={}, topic={}, request={} bytes)",
            handle, wapm, topic, request_len
        );
    }

    let request = thread
        .memory()
        .uint8view_with_byte_offset_and_length(request.offset(), request_len)
        .to_vec();

    // Grab references to the ABI that will be used
    let error_callback = thread.wasm_bus_error_ref();
    let malloc_callback = thread.wasm_bus_malloc_ref();
    let data_callback = thread.wasm_bus_data_ref();
    if error_callback.is_none() || malloc_callback.is_none() || data_callback.is_none() {
        debug!("wasm-bus::call-reply (incorrect abi)");
        return CallError::IncorrectAbi.into();
    }
    let error_callback = error_callback.unwrap().clone();
    let malloc_callback = malloc_callback.unwrap().clone();
    let data_callback = data_callback.unwrap().clone();

    // If its got a parent then we already have an active stream here so we need
    // to feed these results into that stream
    let mut invoke = if let Some(parent) = parent {
        if let Some(session) = thread.inner.unwrap().factory.get(parent) {
            session.call(topic.as_ref(), &request)
        } else {
            ErrornousInvokable::new(CallError::InvalidHandle)
        }
    } else {
        thread.inner.unwrap().factory.start(handle.into(), &wapm, &topic, &request)
    };

    // Invoke the send operation
    let invoke = {
        let thread = thread.clone();
        async move {
            let response = invoke.process().await;

            thread.inner.unwrap().factory.close(CallHandle::from(handle));

            match response {
                Ok(data) => {
                    debug!(
                        "wasm-bus::call-reply (handle={}, response={} bytes)",
                        handle,
                        data.len()
                    );

                    let buf_len = data.len() as u32;
                    let buf = malloc_callback.call(buf_len).unwrap();

                    thread
                        .memory()
                        .uint8view_with_byte_offset_and_length(buf.offset(), buf_len)
                        .copy_from(&data[..]);

                    data_callback.call(handle, buf, buf_len).unwrap();
                }
                Err(err) => {
                    debug!("wasm-bus::call-reply (handle={}, error={})", handle, err);
                    error_callback.call(handle, err.into()).unwrap();
                }
            }
        }
    };

    // We try to invoke the callback synchronously but if it
    // does not complete in time then we add it to the idle
    // processing list which will pick it up again the next time
    // the WASM process yields CPU execution.
    let waker = dummy_waker::dummy_waker();
    let mut cx = Context::from_waker(&waker);
    let mut invoke = Box::pin(invoke);
    if let Poll::Pending = invoke.as_mut().poll(&mut cx) {
        let mut inner = thread.inner.unwrap();
        inner.invocations.insert(handle, invoke);
    }

    // Success
    CallError::Success.into()
}

unsafe fn wasm_bus_yield_and_wait(thread: &WasmBusThread, timeout_ms: u32) {
    trace!("wasm-bus::yield_and_wait (timeout={} ms)", timeout_ms);

    let start = Instant::now();
    loop {
        raw::wasm_bus_tick(thread);
        let elapsed = (Instant::now() - start).as_millis() as u32;
        if elapsed.ge(&timeout_ms) {
            break;
        }
        std::thread::yield_now();
    }
}

unsafe fn wasm_bus_thread_id(thread: &WasmBusThread) -> u32 {
    trace!("wasm-bus::thread_id (id={})", thread.thread_id);
    thread.thread_id
}
