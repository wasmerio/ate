#![cfg_attr(not(debug_assertions), allow(dead_code, unused_imports, unused_variables))]
#![warn(unused_extern_crates)]

/// You can change the hashing routine with these features
/// - feature = "use_blake3"
/// - feature = "use_sha3"

/// You can change the log file format with these features
/// - feature = "use_version1"
/// - feature = "use_version2"

#[cfg(not(target_arch = "wasm32"))]
pub const HASH_ROUTINE: crypto::HashRoutine = if cfg!(feature = "use_blake3") {
    crypto::HashRoutine::Blake3
} else if cfg!(feature = "use_sha3") {
    crypto::HashRoutine::Sha3
} else {
    crypto::HashRoutine::Blake3
};

#[cfg(not(target_arch = "wasm32"))]
pub const LOG_VERSION: spec::EventVersion = spec::EventVersion::V2;

#[cfg(not(target_arch = "wasm32"))]
pub mod utils;
#[cfg(not(target_arch = "wasm32"))]
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod spec;
#[cfg(not(target_arch = "wasm32"))]
pub mod crypto;
#[cfg(not(target_arch = "wasm32"))]
pub mod header;
#[cfg(not(target_arch = "wasm32"))]
pub mod meta;
#[cfg(not(target_arch = "wasm32"))]
pub mod event;
#[cfg(not(target_arch = "wasm32"))]
pub mod conf;
#[cfg(not(target_arch = "wasm32"))]
pub mod comms;
#[cfg(not(target_arch = "wasm32"))]
pub mod mesh;
#[cfg(not(target_arch = "wasm32"))]
pub mod redo;
#[cfg(not(target_arch = "wasm32"))]
pub mod sink;
#[cfg(not(target_arch = "wasm32"))]
pub mod session;
#[cfg(not(target_arch = "wasm32"))]
pub mod validator;
#[cfg(not(target_arch = "wasm32"))]
pub mod compact;
#[cfg(not(target_arch = "wasm32"))]
pub mod index;
#[cfg(not(target_arch = "wasm32"))]
pub mod lint;
#[cfg(not(target_arch = "wasm32"))]
pub mod loader;
#[cfg(not(target_arch = "wasm32"))]
pub mod transform;
#[cfg(not(target_arch = "wasm32"))]
pub mod plugin;
#[cfg(not(target_arch = "wasm32"))]
pub mod signature;
#[cfg(not(target_arch = "wasm32"))]
pub mod time;
#[cfg(not(target_arch = "wasm32"))]
pub mod tree;
#[cfg(not(target_arch = "wasm32"))]
pub mod trust;
#[cfg(not(target_arch = "wasm32"))]
pub mod chain;
#[cfg(not(target_arch = "wasm32"))]
pub mod single;
#[cfg(not(target_arch = "wasm32"))]
pub mod multi;
#[cfg(not(target_arch = "wasm32"))]
pub mod transaction;
#[cfg(not(target_arch = "wasm32"))]
pub mod dio;
#[cfg(not(target_arch = "wasm32"))]
pub mod service;
#[cfg(not(target_arch = "wasm32"))]
pub mod pipe;
#[cfg(not(target_arch = "wasm32"))]
pub mod prelude;
#[cfg(not(target_arch = "wasm32"))]
pub mod anti_replay;
#[cfg(not(target_arch = "wasm32"))]
pub mod flow;
#[cfg(not(target_arch = "wasm32"))]
pub mod repository;