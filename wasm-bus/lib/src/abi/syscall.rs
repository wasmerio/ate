use super::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

fn convert_format(a: wasi::BusDataFormat) -> SerializationFormat {
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

fn convert_format_back(a: SerializationFormat) -> wasi::BusDataFormat {
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

fn convert_err(val: wasi::BusError) -> BusError {
    use BusError::*;
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

fn convert_err_back(val: BusError) -> wasi::BusError {
    use BusError::*;
    match val {
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

fn convert_topic(topic_ptr: *mut u8, topic_len: usize) -> &'static str {
    unsafe {
        // The operating system will resubmit the topic buffer rather than keep allocating it
        // thus the receiver should not free the buffer
        let buf = std::slice::from_raw_parts(topic_ptr, topic_len);
        std::str::from_utf8_unchecked(buf) as &'static str
    }
}

/// Function used to allocate memory during operations like polling
#[no_mangle]
pub extern "C" fn _bus_malloc(len: u64) -> u64 {
    trace!("bus_malloc (len={})", len);
    let mut buf = Vec::with_capacity(len as usize);
    let ptr: *mut u8 = buf.as_mut_ptr();
    std::mem::forget(buf);
    return ptr as u64;
}

/// Callback thats invoked whenever the main BUS needs to do some work
#[no_mangle]
pub extern "C" fn _bus_work(_user_data: u64)
{
    crate::rt::RUNTIME.tick();
}

pub fn bus_poll_once() -> usize {
    let timeout: wasi::Timestamp = 0;
    
    // Read all the events
    let mut events = [wasi::BusEvent {
        tag: wasi::BUS_EVENT_TYPE_NOOP.raw(),
        u: wasi::BusEventU {
            noop: 0
        }
    }; 50];
    let events = unsafe {
        let events_len = events.len();
        let events_ptr = events.as_mut_ptr();
        match wasi::bus_poll(timeout, events_ptr, events_len, "_bus_malloc") {
            Ok(nevents) => {
                // No more events to process
                if nevents <= 0 {
                    return 0;
                }
                &events[..nevents]        
            },
            Err(err) => {
                debug!("failed to poll the bus for events - {}", err.message());
                return 0;
            }
        }
    };
    let nevents = events.len();

    // The blocking guard is to prevent blocking as the loop that called
    // this function is already blocking hence it would deadlock.
    let _blocking_guard = crate::task::blocking_guard();

    // Process the event
    for event in events {
        match event.tag.into() {
            wasi::BUS_EVENT_TYPE_NOOP => { }
            wasi::BUS_EVENT_TYPE_EXIT => {
                // The process these calls relate to has exited
                unsafe {
                    let bid = event.u.exit.bid;
                    let code = event.u.exit.rval;
                    debug!("sub-process ({}) exited with code: {}", bid, code);
                }
            }
            wasi::BUS_EVENT_TYPE_CALL => {
                let handle: CallHandle = unsafe { event.u.call.cid.into() };
                let topic = unsafe {
                    convert_topic(
                        event.u.call.topic,
                        event.u.call.topic_len
                    )
                };
                let request = unsafe {
                    let buf_ptr = event.u.call.buf;
                    let buf_len = event.u.call.buf_len;
                    Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize)
                };
                let parent: Option<CallHandle> = unsafe {
                    match event.u.call.parent.tag.into() {
                        wasi::OPTION_SOME => Some(event.u.call.parent.u.some.into()),
                        wasi::OPTION_NONE | _ => None,
                    }
                };
                let format = unsafe { convert_format(event.u.call.format) };

                trace!(
                    "wasm_bus_start (parent={:?}, handle={}, topic={}, request={} bytes)",
                    parent,
                    handle,
                    topic,
                    request.len()
                );
                if let Err(err) = crate::engine::BusEngine::start(topic.into(), parent, handle, request, format) {
                    call_fault(handle.into(), err);
                }
            }
            wasi::BUS_EVENT_TYPE_RESULT => {
                let handle: CallHandle = unsafe { event.u.result.cid.into() };
                let response = unsafe {
                    let buf_ptr = event.u.result.buf;
                    let buf_len = event.u.result.buf_len;
                    Vec::from_raw_parts(buf_ptr as *mut u8, buf_len as usize, buf_len as usize)
                };
                let format = unsafe { convert_format(event.u.result.format) };
                crate::engine::BusEngine::result(handle, response, format);
            }
            wasi::BUS_EVENT_TYPE_FAULT => {
                let handle: CallHandle = unsafe { event.u.fault.cid.into() };
                let error = unsafe { convert_err(event.u.fault.fault) };
                crate::engine::BusEngine::error(handle, error);
            }
            wasi::BUS_EVENT_TYPE_CLOSE => {
                let handle: CallHandle = unsafe { event.u.close.cid.into() };
                crate::engine::BusEngine::close(&handle, "os_notification");
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
    
    // Returns the number of events that were processed
    nevents
}

pub fn bus_open_local(
    name: &str,
    resuse: bool,
) -> Result<BusHandle, BusError> {
    let reuse = if resuse { wasi::BOOL_TRUE } else { wasi::BOOL_FALSE };
    let ret = unsafe {
        wasi::bus_open_local(
            name,
            reuse
        )
    };
    ret
        .map(|a| a.into())
        .map_err(convert_err)
}

pub fn bus_open_remote(
    name: &str,
    resuse: bool,
    instance: &str,
    token: &str,
) -> Result<BusHandle, BusError> {
    let reuse = if resuse { wasi::BOOL_TRUE } else { wasi::BOOL_FALSE };
    let ret = unsafe {
        wasi::bus_open_remote(
            name,
            reuse,
            instance,
            token
        )
    };
    ret
        .map(|a| a.into())
        .map_err(convert_err)
}

pub fn bus_call(
    bid: BusHandle,
    keepalive: bool,
    topic: &str,
    request: &[u8],
    format: SerializationFormat
) -> Result<CallHandle, BusError> {
    let bid: wasi::Bid = bid.into();
    let keepalive = if keepalive { wasi::BOOL_TRUE } else { wasi::BOOL_FALSE };
    let format = convert_format_back(format);        
    let ret = unsafe {
        wasi::bus_call(
            bid,
            keepalive,
            topic,
            format,
            request
        )
    };

    ret
        .map(|a| a.into())
        .map_err(convert_err)
}

pub fn bus_subcall(
    parent: CallHandle,
    keepalive: bool,
    topic: &str,
    request: &[u8],
    format: SerializationFormat
) -> Result<CallHandle, BusError> {
    let parent = parent.into();
    let keepalive = if keepalive { wasi::BOOL_TRUE } else { wasi::BOOL_FALSE };
    let format = convert_format_back(format);
    let ret = unsafe {
        wasi::bus_subcall(
            parent,
            keepalive,
            topic,
            format,
            request
        )
    };

    ret
        .map(|a| a.into())
        .map_err(convert_err)
    
}

pub fn call_close(handle: CallHandle) {
    unsafe {
        wasi::call_close(handle.into());
    }
}

pub fn call_fault(handle: CallHandle, error: BusError) {
    unsafe {
        let error = convert_err_back(error);
        wasi::call_fault(
            handle.into(),
            error
        );
    }
}

pub fn call_reply(
    handle: CallHandle,
    response: &[u8],
    format: SerializationFormat
) {
    let format = convert_format_back(format);
    unsafe {
        if let Err(err)
            = wasi::call_reply(
                handle.into(),
                format,
                response
            )
        {
            debug!("call reply ({}) failed - {}", handle, err.message())
        }
    }
}

pub fn spawn_reactor() {
    unsafe {
        wasi::thread_spawn(
            "_bus_work",
            0,
            wasi::BOOL_TRUE
        ).unwrap();
    }
}
