#![allow(unused_variables)]
use super::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

pub fn bus_poll_once() -> usize {
    0
}

pub fn bus_open_local(
    name: &str,
    resuse: bool,
) -> Result<BusHandle, BusError> {
    panic!("unsupported on this platform");
}

pub fn bus_open_remote(
    name: &str,
    resuse: bool,
    instance: &str,
    token: &str,
) -> Result<BusHandle, BusError> {
    panic!("unsupported on this platform");
}

pub fn bus_call(
    bid: BusHandle,
    keepalive: bool,
    topic_hash: u128,
    request: &[u8],
    format: SerializationFormat
) -> Result<CallHandle, BusError> {
    panic!("unsupported on this platform");
}

pub fn bus_subcall(
    parent: CallHandle,
    keepalive: bool,
    topic_hash: u128,
    request: &[u8],
    format: SerializationFormat
) -> Result<CallHandle, BusError> {
    panic!("unsupported on this platform");
}

pub fn call_close(handle: CallHandle) {
    panic!("unsupported on this platform");
}

pub fn call_fault(handle: CallHandle, error: BusError) {
    panic!("unsupported on this platform");
}

pub fn call_reply(
    handle: CallHandle,
    response: &[u8],
    format: SerializationFormat
) {
    panic!("unsupported on this platform");
}

pub fn spawn_reactor() {
    panic!("unsupported on this platform");
}
