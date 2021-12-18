#![allow(dead_code)]
use super::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

pub fn drop(_handle: CallHandle) {
    panic!("unsupported on this platform");
}

pub fn handle() -> CallHandle {
    panic!("unsupported on this platform");
}

pub fn fault(_handle: CallHandle, _error: u32) {
    panic!("unsupported on this platform");
}

pub fn poll() {
    panic!("unsupported on this platform");
}

pub fn listen(_topic: &str) {
    panic!("unsupported on this platform");
}

pub fn reply(_handle: CallHandle, _response: &[u8]) {
    panic!("unsupported on this platform");
}

pub fn call(
    _parent: Option<CallHandle>,
    _handle: CallHandle,
    _wapm: &str,
    _topic: &str,
    _request: &[u8],
) {
    panic!("unsupported on this platform");
}

pub fn callback(_parent: CallHandle, _handle: CallHandle, _topic: &str) {
    panic!("unsupported on this platform");
}

pub fn thread_id() -> u32 {
    panic!("unsupported on this platform");
}
