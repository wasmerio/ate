use crate::api::SystemAbiExt;
use crate::wasmer::Array;
use crate::wasmer::WasmPtr;
use crate::wasmer_wasi::WasiError;
use std::collections::HashMap;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;

use super::thread::WasmBusThread;
use super::*;

pub(crate) mod raw {
    use super::*;
    pub fn wasm_bus_drop(thread: &WasmBusThread, handle: u32) {
        unsafe { super::wasm_bus_drop(thread, handle.into()) }
    }
    pub fn wasm_bus_handle(thread: &WasmBusThread) -> u32 {
        unsafe { super::wasm_bus_handle(thread).into() }
    }
    pub fn wasm_bus_listen(thread: &WasmBusThread, topic_ptr: u32, topic_len: u32) {
        let topic_ptr: WasmPtr<u8, Array> = WasmPtr::new(topic_ptr as u32);
        unsafe { super::wasm_bus_listen(thread, topic_ptr, topic_len as usize) }
    }
    pub fn wasm_bus_callback(
        thread: &WasmBusThread,
        parent: u32,
        handle: u32,
        topic_ptr: u32,
        topic_len: u32,
    ) {
        let parent: Option<CallHandle> = if parent != u32::MAX {
            Some(parent.into())
        } else {
            None
        };
        let handle: CallHandle = handle.into();
        let topic_ptr: WasmPtr<u8, Array> = WasmPtr::new(topic_ptr as u32);
        unsafe { super::wasm_bus_callback(thread, parent, handle, topic_ptr, topic_len as usize) }
    }
    pub fn wasm_bus_fault(thread: &WasmBusThread, handle: u32, error: u32) {
        let handle: CallHandle = handle.into();
        unsafe { super::wasm_bus_fault(thread, handle, error) }
    }
    pub fn wasm_bus_poll(thread: &WasmBusThread) -> Result<(), WasiError> {
        unsafe { super::wasm_bus_poll(thread) }
    }
    pub fn wasm_bus_fork(thread: &WasmBusThread) -> Result<(), WasiError> {
        unsafe { super::wasm_bus_fork(thread) }
    }
    pub fn wasm_bus_reply(
        thread: &WasmBusThread,
        handle: u32,
        response_ptr: u32,
        response_len: u32,
    ) {
        let handle: CallHandle = handle.into();
        let response_ptr: WasmPtr<u8, Array> = WasmPtr::new(response_ptr as u32);
        unsafe { super::wasm_bus_reply(thread, handle, response_ptr, response_len as usize) }
    }
    pub fn wasm_bus_reply_callback(
        thread: &WasmBusThread,
        handle: u32,
        topic_ptr: u32,
        topic_len: u32,
        response_ptr: u32,
        response_len: u32,
    ) {
        let handle: CallHandle = handle.into();
        let topic_ptr: WasmPtr<u8, Array> = WasmPtr::new(topic_ptr as u32);
        let response_ptr: WasmPtr<u8, Array> = WasmPtr::new(response_ptr as u32);
        unsafe {
            super::wasm_bus_reply_callback(
                thread,
                handle,
                topic_ptr,
                topic_len as usize,
                response_ptr,
                response_len as usize,
            )
        }
    }
    pub fn wasm_bus_call(
        thread: &WasmBusThread,
        parent: u32,
        handle: u32,
        wapm_ptr: u32,
        wapm_len: u32,
        topic_ptr: u32,
        topic_len: u32,
        request_ptr: u32,
        request_len: u32,
    ) -> u32 {
        let parent: Option<CallHandle> = if parent != u32::MAX {
            Some(parent.into())
        } else {
            None
        };
        let handle: CallHandle = handle.into();
        let wapm_ptr: WasmPtr<u8, Array> = WasmPtr::new(wapm_ptr as u32);
        let topic_ptr: WasmPtr<u8, Array> = WasmPtr::new(topic_ptr as u32);
        let request_ptr: WasmPtr<u8, Array> = WasmPtr::new(request_ptr as u32);
        unsafe {
            super::wasm_bus_call(
                thread,
                parent,
                handle,
                wapm_ptr,
                wapm_len as usize,
                topic_ptr,
                topic_len as usize,
                request_ptr,
                request_len as usize,
            )
        }
    }
    pub fn wasm_bus_thread_id(thread: &WasmBusThread) -> u32 {
        unsafe { super::wasm_bus_thread_id(thread) }
    }
}

// Drops a handle used by calls or callbacks
pub(crate) unsafe fn wasm_bus_drop(thread: &WasmBusThread, handle: CallHandle) {
    let handle: CallHandle = handle.into();

    let mut delayed_drop1 = Vec::new();
    let mut delayed_drop2 = Vec::new();
    let mut delayed_drop3 = Vec::new();
    {
        let mut inner = thread.inner.lock();
        delayed_drop1.push(inner.invocations.remove(&handle));
        delayed_drop2.push(inner.callbacks.remove(&handle));
        delayed_drop3.push(inner.factory.close(handle));
    }
}

unsafe fn wasm_bus_handle(_thread: &WasmBusThread) -> CallHandle {
    fastrand::u32(..).into()
}

// Incidates that a call that will be made should invoke a callback
// back to this process under the designated handle.
unsafe fn wasm_bus_callback(
    thread: &WasmBusThread,
    parent: Option<CallHandle>,
    handle: CallHandle,
    topic_ptr: WasmPtr<u8, Array>,
    topic_len: usize,
) {
    let topic = topic_ptr
        .get_utf8_str(thread.memory(), topic_len as u32)
        .unwrap();
    debug!(
        "wasm-bus::recv (parent={:?}, handle={}, topic={})",
        parent, handle.id, topic
    );

    let mut inner = thread.inner.lock();
    if let Some(parent) = parent {
        let entry = inner.callbacks.entry(parent).or_default();
        entry.insert(topic.to_string(), handle);
        return;
    }
}

// Forks the current process and then continuously polls the operating system
// for new work and/or messages which will be returned via the 'wasm_bus_start'
// (and friends) function calls.
unsafe fn wasm_bus_fork(thread: &WasmBusThread) -> Result<(), WasiError> {
    trace!("wasm-bus::fork");

    // For the ability to poll for work we must take the receiver side of
    // a work queue - there is only one receiver so poll can only be
    // called once - this library will call back into the WASM module from there
    let work_rx = {
        let mut inner = thread.inner.lock();
        inner.work_rx.take()
    };
    if let Some(mut work_rx) = work_rx {
        // Register the polling thread so that it can be picked up by the
        // main WASM thread after it exits
        debug!("wasm-bus::fork - registering the polling thread");
        {
            let thread_inside = thread.clone();
            let worker = Box::pin(async move {
                // Set the polling flag
                {
                    let inner = thread_inside.inner.lock();
                    let _ = inner.polling.send(true);
                }

                // We pass the thread object into and out of the dedicated thread which
                // gives access to the thread callbacks without allowing for situations
                // of re-entrance (this half is the side inside the main
                // wasm thread)
                let mut ret = crate::err::ERR_OK;
                while let Some(work) = work_rx.recv().await {
                    ret = thread_inside.work(work).await;
                    if ret != crate::err::ERR_OK {
                        break;
                    }
                }
                debug!("wasm-bus::fork - worker has exited");

                // We return the worker queue as we no longer need it
                let mut inner = thread_inside.inner.lock();
                inner.work_rx.replace(work_rx);
                ret
            });

            thread.inner.lock().poll_thread.replace(worker);
        }
        // Now we exit the main thread (anything that is not a global
        // variable will be lost)
        debug!("wasm-bus::fork - exiting from main");
        return Ok(());
    }

    // We have a duplicate poll call (either from within a poll call
    // or a second invocation of it. Given the reciever queue is
    // already consumed this would cause a deadlock or multithreading issues
    warn!("wasm-bus::fork failed - fork queue already consumed");
    return Err(WasiError::Exit(crate::err::ERR_EDEADLK));
}

// Polls the operating system for any pending callback responses
unsafe fn wasm_bus_poll(thread: &WasmBusThread) -> Result<(), WasiError> {
    trace!("wasm-bus::poll");

    let mut wait_time = 0u64;
    loop {
        if wasm_bus_tick(thread) > 0 {
            break;
        }
        if let Some(exit_code) = thread.ctx.should_terminate() {
            return Err(WasiError::Exit(exit_code));
        }
        // Linearly increasing wait time
        wait_time += 1;
        let wait_time = u64::min(wait_time / 10, 20);
        std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
    }
    Ok(())
}

unsafe fn wasm_bus_tick(thread: &WasmBusThread) -> usize {
    thread.process()
}

// Tells the operating system that this program is ready to respond
// to calls on a particular topic name.
unsafe fn wasm_bus_listen(thread: &WasmBusThread, topic_ptr: WasmPtr<u8, Array>, topic_len: usize) {
    let topic = topic_ptr
        .get_utf8_str(thread.memory(), topic_len as u32)
        .unwrap();
    debug!("wasm-bus::listen (topic={})", topic);

    let mut inner = thread.inner.lock();
    inner.listens.insert(topic.to_string());
}

// Indicates that a fault has occured while processing a call
unsafe fn wasm_bus_fault(thread: &WasmBusThread, handle: CallHandle, error: u32) {
    use tokio::sync::mpsc::error::TrySendError;

    debug!("wasm-bus::error (handle={}, error={})", handle.id, error);

    // Grab the sender we will relay this response to
    let error: CallError = error.into();
    let work = {
        let mut inner = thread.inner.lock();
        inner.calls.remove(&handle)
    };
    if let Some(work) = work {
        if let Err(err) = work.try_send(Err(error)) {
            let response = match err {
                TrySendError::Closed(a) => a,
                TrySendError::Full(a) => a,
            };
            thread.system.task_shared(Box::new(move || {
                Box::pin(async move {
                    let _ = work.send(response).await;
                })
            }));
        }
    }
}

// Returns the response of a listen invokation to a program
// from the operating system
unsafe fn wasm_bus_reply(
    thread: &WasmBusThread,
    handle: CallHandle,
    response_ptr: WasmPtr<u8, Array>,
    response_len: usize,
) {
    use tokio::sync::mpsc::error::TrySendError;

    debug!(
        "wasm-bus::reply (handle={}, response={} bytes)",
        handle.id, response_len
    );

    // Grab the data we are sending back
    let response = thread
        .memory()
        .uint8view_with_byte_offset_and_length(response_ptr.offset(), response_len as u32)
        .to_vec();

    // Grab the sender we will relay this response to
    let work = {
        let mut inner = thread.inner.lock();
        inner.calls.remove(&handle)
    };
    if let Some(work) = work {
        if let Err(err) = work.try_send(Ok(response)) {
            let response = match err {
                TrySendError::Closed(a) => a,
                TrySendError::Full(a) => a,
            };
            thread.system.task_shared(Box::new(move || {
                Box::pin(async move {
                    let _ = work.send(response).await;
                })
            }));
        }
    }
}

// Returns the response of a listen callback
unsafe fn wasm_bus_reply_callback(
    thread: &WasmBusThread,
    handle: CallHandle,
    topic_ptr: WasmPtr<u8, Array>,
    topic_len: usize,
    response_ptr: WasmPtr<u8, Array>,
    response_len: usize,
) {
    let topic = topic_ptr
        .get_utf8_str(thread.memory(), topic_len as u32)
        .unwrap()
        .to_string();
    debug!(
        "wasm-bus::reply_callback (handle={}, topic={}, response={} bytes)",
        handle.id, topic, response_len
    );

    // Grab the data we are sending back
    let response = thread
        .memory()
        .uint8view_with_byte_offset_and_length(response_ptr.offset(), response_len as u32)
        .to_vec();

    // Grab the callback this related to
    let callback = {
        let inner = thread.inner.lock();
        inner
            .callbacks
            .get(&handle)
            .map(|handle| handle.get(&topic))
            .flatten()
            .map(|handle| WasmBusFeeder::new(thread, handle.clone()))
    };

    // Grab the sender we will relay this response to
    if let Some(callback) = callback {
        callback.feed_bytes(response);
    } else {
        debug!("callback is lost (topic={})", topic);
    }
}

// Calls a function using the operating system call to find
// the right target based on the wapm and topic.
// The operating system will respond with either a 'wasm_bus_finish'
// or a 'wasm_bus_error' message.
unsafe fn wasm_bus_call(
    thread: &WasmBusThread,
    parent: Option<CallHandle>,
    handle: CallHandle,
    wapm_ptr: WasmPtr<u8, Array>,
    wapm_len: usize,
    topic_ptr: WasmPtr<u8, Array>,
    topic_len: usize,
    request_ptr: WasmPtr<u8, Array>,
    request_len: usize,
) -> u32 {
    let wapm = wapm_ptr
        .get_utf8_str(thread.memory(), wapm_len as u32)
        .unwrap();
    let topic = topic_ptr
        .get_utf8_str(thread.memory(), topic_len as u32)
        .unwrap();
    if let Some(parent) = parent {
        debug!(
            "wasm-bus::call (parent={}, handle={}, wapm={}, topic={}, request={} bytes)",
            parent.id, handle.id, wapm, topic, request_len
        );
    } else {
        debug!(
            "wasm-bus::call (handle={}, wapm={}, topic={}, request={} bytes)",
            handle.id, wapm, topic, request_len
        );
    }

    let request = thread
        .memory()
        .uint8view_with_byte_offset_and_length(request_ptr.offset(), request_len as u32)
        .to_vec();

    // Grab references to the ABI that will be used
    let data_feeder = WasmBusFeeder::new(thread, handle.into());

    // Grab all the client callbacks that have been registered
    let client_callbacks: HashMap<String, WasmBusFeeder> = {
        let mut inner = thread.inner.lock();
        inner
            .callbacks
            .remove(&handle)
            .map(|a| {
                a.into_iter()
                    .map(|(topic, handle)| (topic, WasmBusFeeder::new(thread, handle.into())))
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut invoke = {
        let mut inner = thread.inner.lock();
        inner.factory.start(
            thread,
            parent,
            handle.into(),
            wapm.to_string(),
            topic.to_string(),
            request,
            client_callbacks,
            thread.ctx.clone(),
        )
    };

    // Invoke the send operation
    let (abort_tx, mut abort_rx) = mpsc::channel(1);
    let result = {
        let thread = thread.clone();
        thread.system.spawn_shared(move || async move {
            tokio::select! {
                response = invoke.process() => {
                    response
                }
                _ = abort_rx.recv() => {
                    Err(CallError::Aborted)
                }
            }
        })
    };

    // Turn it into a invocations object
    let invoke = WasmBusThreadInvocation {
        _abort: abort_tx,
        result,
        data_feeder,
    };

    // Record the invocations and return success
    let mut inner = thread.inner.lock();
    inner.invocations.insert(handle, invoke);
    CallError::Success.into()
}

// Returns a unqiue ID for the thread
unsafe fn wasm_bus_thread_id(thread: &WasmBusThread) -> u32 {
    trace!("wasm-bus::thread_id (id={})", thread.thread_id);
    thread.thread_id
}
