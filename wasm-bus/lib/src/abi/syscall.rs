#![allow(dead_code)]
use std::mem::ManuallyDrop;

use super::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasi::BusError;

/*
mod raw {
    use super::*;

    // Frees memory that was passed to the operating system by
    // the program
    #[no_mangle]
    pub extern "C" fn wasm_bus_free(buf_ptr: u32, buf_len: u32) {
        trace!("wasm_bus_free (buf={} bytes)", buf_len);
        unsafe {
            let data = Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize);
            std::mem::drop(data);
        }
    }

    // Allocates memory that will be used to pass data from the
    // operating system back to this program
    #[no_mangle]
    pub extern "C" fn wasm_bus_malloc(len: u32) -> u32 {
        trace!("wasm_bus_malloc (len={})", len);
        let mut buf = Vec::with_capacity(len as usize);
        let ptr: *mut u8 = buf.as_mut_ptr();
        std::mem::forget(buf);
        return ptr as u32;
    }

    // Invoked by the operating system when during a poll when a
    // request is to be processed by this program
    #[no_mangle]
    pub extern "C" fn wasm_bus_start(
        parent: u32,
        handle: u32,
        topic_ptr: u32,
        topic_len: u32,
        request_ptr: u32,
        request_len: u32,
    ) {
        let parent = match parent {
            u32::MAX => None,
            a => Some(a.into()),
        };

        let topic = unsafe {
            let topic =
                Vec::from_raw_parts(topic_ptr as *mut u8, topic_len as usize, topic_len as usize);
            String::from_utf8_lossy(&topic[..]).to_string()
        };
        trace!(
            "wasm_bus_start (parent={:?}, handle={}, topic={}, request={} bytes)",
            parent,
            handle,
            topic,
            request_len
        );

        // The blocking guard is to prevent blocking as the loop that called
        // this function is already blocking hence it would deadlock.
        let _blocking_guard = crate::task::blocking_guard();

        unsafe {
            let request = Vec::from_raw_parts(
                request_ptr as *mut u8,
                request_len as usize,
                request_len as usize,
            );

            let handle: CallHandle = handle.into();
            if let Err(err) = crate::engine::BusEngine::start(topic, parent, handle, request) {
                fault(handle.into(), err as u32);
            }
        }

        // This function is the one that actually processing the call but it will
        // not nessasarily complete the call in one go - if it idles then thats
        // because its waiting for something else from the wasm_bus hence we return
        #[cfg(feature = "rt")]
        crate::task::wake();
        #[cfg(feature = "rt")]
        crate::task::work_it();
    }

    // Invoked by the operating system when a call has finished
    // This call includes the data that was returned
    #[no_mangle]
    pub extern "C" fn wasm_bus_finish(handle: u32, data: u32, data_len: u32) {
        unsafe {
            let response =
                Vec::from_raw_parts(data as *mut u8, data_len as usize, data_len as usize);

            crate::engine::BusEngine::finish(handle.into(), response);
        }

        #[cfg(feature = "rt")]
        crate::task::wake();
        #[cfg(feature = "rt")]
        crate::task::work_it();
    }

    // Invoked by the operating system when the call this program made has failed
    #[no_mangle]
    pub extern "C" fn wasm_bus_error(handle: u32, error: u32) {
        crate::engine::BusEngine::error(handle.into(), error.into());

        #[cfg(feature = "rt")]
        crate::task::wake();
        #[cfg(feature = "rt")]
        crate::task::work_it();
    }

    // Invoked by the operating system when a call has been terminated by the caller
    #[no_mangle]
    pub extern "C" fn wasm_bus_drop(handle: u32) {
        let handle: CallHandle = handle.into();
        crate::engine::BusEngine::remove(&handle, "os_notification");

        #[cfg(feature = "rt")]
        crate::task::wake();
        #[cfg(feature = "rt")]
        crate::task::work_it();
    }

    #[link(wasm_import_module = "wasm-bus")]
    extern "C" {
        // Returns a handle 64-bit number which is used while generating
        // handles for calls and receive hooks
        pub(crate) fn handle() -> u32;

        // Indicates that a fault has occured while processing a call
        pub(crate) fn fault(handle: u32, error: u32);

        // Returns the response of a listen invokation to a program
        // from the operating system
        pub(crate) fn reply(handle: u32, response: u32, response_len: u32);

        // Call thats made when a sub-process is making a callback of
        // a particular type
        pub(crate) fn reply_callback(
            handle: u32,
            topic: u32,
            topic_len: u32,
            request: u32,
            request_len: u32,
        );

        // Drops a handle used by calls or callbacks
        pub(crate) fn drop(handle: u32);

        // Calls a function using the operating system call to find
        // the right target based on the wapm and topic.
        // The operating system will respond with either a 'wasm_bus_finish'
        // or a 'wasm_bus_error' message.
        pub(crate) fn call(
            parent: u32,
            handle: u32,
            keepalive: u32,
            wapm: u32,
            wapm_len: u32,
            topic: u32,
            topic_len: u32,
            request: u32,
            request_len: u32,
        ) -> u32;

        // Calls a function within a hosted instance 
        // The operating system will respond with either a 'wasm_bus_finish'
        // or a 'wasm_bus_error' message.
        pub(crate) fn call_instance(
            parent: u32,
            handle: u32,
            keepalive: u32,
            instance: u32,
            instance_len: u32,
            access_token: u32,
            access_token_len: u32,
            wapm: u32,
            wapm_len: u32,
            topic: u32,
            topic_len: u32,
            request: u32,
            request_len: u32,
        ) -> u32;

        // Incidates that a call that will be made should invoke a callback
        // back to this process under the designated handle.
        pub(crate) fn callback(parent: u32, handle: u32, topic: u32, topic_len: u32);

        // Tells the operating system that this program is ready to respond
        // to calls on a particular topic name.
        pub(crate) fn listen(topic: u32, topic_len: u32);

        // Polls the operating system for result messages that are the completion
        // events for the calls we made out to the wasm_bus
        pub(crate) fn poll();

        // Forks the process (after the main thread exists) so that it can process
        // any inbound work via 'wasm_bus_start' function call.
        pub(crate) fn fork();

        // Returns a unqiue ID for the thread
        pub(crate) fn thread_id() -> u32;
    }
}
*/

/// Function used to allocate memory during operations like polling
#[no_mangle]
pub extern "C" fn _bus_malloc(len: u64) -> u64 {
    trace!("bus_malloc (len={})", len);
    let mut buf = Vec::with_capacity(len as usize);
    let ptr: *mut u8 = buf.as_mut_ptr();
    std::mem::forget(buf);
    return ptr as u64;
}

// Frees memory that was passed to the operating system by the program
#[no_mangle]
pub extern "C" fn _bus_free(buf_ptr: u64, buf_len: u64) {
    trace!("bus_free (buf={} bytes)", buf_len);
    unsafe {
        let data = Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize);
        std::mem::drop(data);
    }
}

/// Callback thats invoked whenever the main BUS needs to do some work
#[no_mangle]
pub extern "C" fn _bus_work()
{
    match poll(None) {
        Ok(()) => { },
        Err(err) => {
            debug!("bus-work-failed: {}", err.message());
        }
    }    
}

impl Into<wasi::BusDataFormat>
for SerializationFormat
{
    fn into(a: Self) -> wasi::BusDataFormat {
        use SerializationFormat::*;

        match a {
            Bincode => wasi::BUS_DATA_FORMAT_BINCODE,
            MessagePack => wasi::BUS_DATA_FORMAT_MESSAGE_PACK,
            Json => wasi::BUS_DATA_FORMAT_JSON,
            Yaml => wasi::BUS_DATA_FORMAT_YAML,
            Xml => wasi::BUS_DATA_FORMAT_XML,
            Raw => wasi::BUS_DATA_FORMAT_RAW
        }
    }
}

impl From<wasi::BusDataFormat>
for SerializationFormat
{
    fn from(a: wasi::BusDataFormat) -> Self {
        use SerializationFormat::*;

        match a {
            wasi::BUS_DATA_FORMAT_BINCODE => Bincode,
            wasi::BUS_DATA_FORMAT_MESSAGE_PACK => MessagePack,
            wasi::BUS_DATA_FORMAT_JSON => Json,
            wasi::BUS_DATA_FORMAT_YAML => Yaml,
            wasi::BUS_DATA_FORMAT_XML => Xml,
            wasi::BUS_DATA_FORMAT_RAW | _ => Raw
        }
    }
}

impl Into<wasi::BusDataFormat>
for SerializationFormat
{
    fn into(a: Self) -> wasi::BusDataFormat {
        use SerializationFormat::*;

        match a {
            Bincode => wasi::BUS_DATA_FORMAT_BINCODE,
            MessagePack => wasi::BUS_DATA_FORMAT_MESSAGE_PACK,
            Json => wasi::BUS_DATA_FORMAT_JSON,
            Yaml => wasi::BUS_DATA_FORMAT_YAML,
            Xml => wasi::BUS_DATA_FORMAT_XML,
            Raw => wasi::BUS_DATA_FORMAT_RAW
        }
    }
}

impl From<wasi::BusError> for CallError {
    fn from(val: wasi::BusError) -> CallError {
        use CallError::*;
        match val {
            wasi::BUS_ERROR_SUCCESS => Success,
            wasi::BUS_ERROR_SERIALIZATION => SerializationFailed,
            wasi::BUS_ERROR_DESERIALIZATION => DeserializationFailed,
            wasi::BUS_ERROR_INVALID_WAPM => InvalidWapm,
            wasi::BUS_ERROR_FETCH_WAPM => FetchFailed,
            wasi::BUS_ERROR_COMPILE_ERROR => CompileError,
            wasi::BUS_ERROR_INVALID_ABI => IncorrectAbi,
            wasi::BUS_ERROR_ABORTED => Aborted,
            wasi::BUS_ERROR_INVALID_HANDLE => InvalidHandle,
            wasi::BUS_ERROR_INVALID_TOPIC => InvalidTopic,
            wasi::BUS_ERROR_MISSING_CALLBACK => MissingCallbacks,
            wasi::BUS_ERROR_UNSUPPORTED => Unsupported,
            wasi::BUS_ERROR_BAD_REQUEST => BadRequest,
            wasi::BUS_ERROR_ACCESS_DENIED => AccessDenied,
            wasi::BUS_ERROR_INTERNAL_FAILURE => InternalFailure,
            wasi::BUS_ERROR_MEMORY_ALLOCATION_FAILED => MemoryAllocationFailed,
            wasi::BUS_ERROR_BUS_INVOCATION_FAILED => BusInvocationFailed,
            wasi::BUS_ERROR_ALREADY_CONSUMED => AlreadyConsumed,
            wasi::BUS_ERROR_MEMORY_ACCESS_VIOLATION => MemoryAccessViolation,
            wasi::BUS_ERROR_UNKNOWN_ERROR | _ => Unknown,
        }
    }
}

impl Into<wasi::BusError> for CallError {
    fn into(self) -> wasi::BusError {
        use CallError::*;
        match self {
            Success => wasi::BUS_ERROR_SUCCESS,
            SerializationFailed => wasi::BUS_ERROR_SERIALIZATION,
            DeserializationFailed => wasi::BUS_ERROR_DESERIALIZATION,
            InvalidWapm => wasi::BUS_ERROR_INVALID_WAPM,
            FetchFailed => wasi::BUS_ERROR_FETCH_WAPM,
            CompileError => wasi::BUS_ERROR_COMPILE_ERROR,
            IncorrectAbi => wasi::BUS_ERROR_INVALID_ABI,
            Aborted => wasi::BUS_ERROR_ABORTED,
            InvalidHandle => wasi::BUS_ERROR_INVALID_HANDLE,
            InvalidTopic => wasi::BUS_ERROR_INVALID_TOPIC,
            MissingCallbacks => wasi::BUS_ERROR_MISSING_CALLBACK,
            Unsupported => wasi::BUS_ERROR_UNSUPPORTED,
            BadRequest => wasi::BUS_ERROR_BAD_REQUEST,
            AccessDenied => wasi::BUS_ERROR_ACCESS_DENIED,
            InternalFailure => wasi::BUS_ERROR_INTERNAL_FAILURE,
            MemoryAllocationFailed => wasi::BUS_ERROR_MEMORY_ALLOCATION_FAILED,
            BusInvocationFailed => wasi::BUS_ERROR_BUS_INVOCATION_FAILED,
            AlreadyConsumed => wasi::BUS_ERROR_ALREADY_CONSUMED,
            MemoryAccessViolation => wasi::BUS_ERROR_MEMORY_ACCESS_VIOLATION,
            Unknown => wasi::BUS_ERROR_UNKNOWN_ERROR
        }
    }
}

pub fn poll(bid: Option<BusHandle>) -> Result<(), wasi::BusError> {
    let bid = match bid {
        None => wasi::OptionBid {
            tag: wasi::OPTION_NONE.raw(),
            u: wasi::OptionBidU {
                none: 0,
            }
        },
        Some(bid) => wasi::OptionBid {
            tag: wasi::OPTION_SOME.raw(),
            u: wasi::OptionBidU {
                some: bid.id.into()
            }
        }
    };
    let timeout: wasi::Timestamp = 0;
    loop {
        // Read all the events
        let mut events = [wasi::BusEvent {
            tag: wasi::BUS_EVENT_TYPE_NOOP.raw(),
            u: wasi::BusEventU {
                noop: 0
            }
        }; 50];
        let events = unsafe {
            let bid = &bid as *const wasi::OptionBid;
            let events_len = events.len();
            let events_ptr = events.as_mut_ptr();
            let nevents = wasi::bus_poll(bid, timeout, events_ptr, events_len, "_bus_malloc")?;
                
            // No more events to process
            if nevents <= 0 {
                break;
            }
            &events[..nevents]
        };

        // The blocking guard is to prevent blocking as the loop that called
        // this function is already blocking hence it would deadlock.
        let _blocking_guard = crate::task::blocking_guard();

        // Process the event
        for event in events {
            match event.tag.into() {
                wasi::BUS_EVENT_TYPE_NOOP => { }
                wasi::BUS_EVENT_TYPE_EXIT => {
                    let code = event.u.exit.rval;
                    std::process::exit(code as i32);
                }
                wasi::BUS_EVENT_TYPE_CALL => {
                    let handle: CallHandle = event.u.call.cid.into();
                    let topic = unsafe {
                        let buf_ptr = event.u.call.topic;
                        let buf_len = event.u.call.topic_len;
                        // The operating system will resubmit the topic buffer rather than keep allocating it
                        // thus the receiver should not free the buffer
                        let buf = ManuallyDrop::new(
                            Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize)
                        );
                        std::str::from_utf8_unchecked(&buf[..]) as &'static str
                    };
                    let request = unsafe {
                        let buf_ptr = event.u.call.buf;
                        let buf_len = event.u.call.buf_len;
                        Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize)
                    };
                    let parent: Option<CallHandle> = match event.u.call.parent.tag.into() {
                        wasi::OPTION_NONE => None,
                        wasi::OPTION_SOME => Some(event.u.call.parent.u.some.into())
                    };
                    let format: SerializationFormat = event.u.call.format.into();

                    trace!(
                        "wasm_bus_start (parent={:?}, handle={}, topic={}, request={} bytes)",
                        parent,
                        handle,
                        topic,
                        request.len()
                    );
                    if let Err(err) = crate::engine::BusEngine::start(topic, parent, handle, request, format) {
                        fault(handle.into(), err as u32);
                    }
                }
                wasi::BUS_EVENT_TYPE_FINISH => {
                    let handle: CallHandle = event.u.finish.cid.into();
                    let response = unsafe {
                        let buf_ptr = event.u.finish.buf;
                        let buf_len = event.u.finish.buf_len;
                        Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize)
                    };
                    let format: SerializationFormat = event.u.finish.format.into();
                    crate::engine::BusEngine::finish(handle, response, format);
                }
                wasi::BUS_EVENT_TYPE_FAULT => {
                    let handle: CallHandle = event.u.fault.cid.into();
                    let error: CallError = event.u.fault.fault.raw().into();
                    crate::engine::BusEngine::error(handle, error);
                }
                wasi::BUS_EVENT_TYPE_DROP => {
                    let handle: CallHandle = event.u.drop.cid.into();
                    crate::engine::BusEngine::remove(&handle, "os_notification");
                }
                a => {
                    debug!("unknown bus event type ({})", a.raw());
                }
            }
        }

        // This function is the one that actually processing the call but it will
        // not necessarily complete the call in one go - if it idles then thats
        // because its waiting for something else from the wasm_bus hence we return
        #[cfg(feature = "rt")]
        crate::task::wake();
        #[cfg(feature = "rt")]
        crate::task::work_it();
    }
    Ok(())
}

pub fn drop(handle: CallHandle) {
    unsafe { wasi::bus_drop(handle.raw()) }
}

pub fn handle() -> CallHandle {
    unsafe { raw::handle().into() }
}

pub fn fault(handle: CallHandle, error: u32) {
    unsafe {
        raw::fault(handle.id, error);
    }
}

pub fn poll() {
    unsafe { raw::poll() }
}

pub fn fork() {
    unsafe { raw::fork() }
}

pub fn listen(topic: &str) {
    unsafe {
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        raw::listen(topic as u32, topic_len as u32)
    }
}

pub fn reply(handle: CallHandle, response: &[u8]) {
    unsafe {
        let response_len = response.len();
        let response = response.as_ptr();
        raw::reply(handle.id, response as u32, response_len as u32);
    }
}

pub fn reply_callback(handle: CallHandle, topic: &str, response: &[u8]) {
    unsafe {
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        let response_len = response.len();
        let response = response.as_ptr();
        raw::reply_callback(
            handle.id,
            topic as u32,
            topic_len as u32,
            response as u32,
            response_len as u32,
        );
    }
}

pub fn call(
    bid: BusHandle,
    parent: Option<CallHandle>,
    keepalive: bool,
    topic: &str,
    request: &[u8],
    format: SerializationFormat
) -> Result<CallHandle, CallError> {
    let bid: wasi::Bid = bid.into();
    let parent = match parent {
        None => wasi::OptionCid {
            tag: wasi::OPTION_NONE.raw(),
            u: wasi::OptionCidU {
                none: 0,
            }
        },
        Some(cid) => wasi::OptionCid {
            tag: wasi::OPTION_SOME.raw(),
            u: wasi::OptionCidU {
                some: cid.id.into()
            }
        }
    };

    let ret = unsafe {
        let parent = &parent as *const wasi::OptionCid;
        let keepalive = if keepalive { wasi::BOOL_TRUE } else { wasi::BOOL_FALSE };
        let format: wasi::BusDataFormat = format.into();
        wasi::bus_invoke(
            bid,
            parent,
            keep_alive,
            topic,
            format,
            request
        )
    };

    ret
        .map(|a| a.into())
        .map_err(|a| a.into())
    
}

pub fn callback(parent: CallHandle, handle: CallHandle, topic: &str) {
    let cid = wasi::Cid = handle.id;
    unsafe {
        let topic_len = topic.len();
        let topic = topic.as_ptr();
        wasi::bus_callback(

        )
        raw::callback(parent.id, handle.id, topic as u32, topic_len as u32)
    }
}

pub fn thread_id() -> u32 {
    unsafe { raw::thread_id() }
}
