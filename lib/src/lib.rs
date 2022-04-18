#![cfg_attr(
    not(debug_assertions),
    allow(dead_code, unused_imports, unused_variables)
)]
#![warn(unused_extern_crates)]

/// You can change the log file format with these features
/// - feature = "use_version1"
/// - feature = "use_version2"

pub const LOG_VERSION: spec::EventVersion = spec::EventVersion::V2;

pub mod anti_replay;
pub mod chain;
pub mod comms;
pub mod compact;
pub mod conf;
pub mod dio;
#[cfg(feature = "enable_dns")]
pub mod dns;
pub mod engine;
pub mod error;
pub mod event;
#[cfg(feature = "enable_server")]
pub mod flow;
pub mod header;
pub mod index;
pub mod lint;
pub mod loader;
pub mod mesh;
pub mod meta;
pub mod multi;
pub mod pipe;
pub mod plugin;
pub mod prelude;
pub mod redo;
pub mod service;
pub mod session;
pub mod signature;
pub mod single;
pub mod sink;
pub mod spec;
pub mod time;
pub mod transaction;
pub mod transform;
pub mod tree;
pub mod trust;
pub mod utils;
pub mod validator;

pub use ate_crypto::crypto;
pub use ate_crypto::utils::log_init;
