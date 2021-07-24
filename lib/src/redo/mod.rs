#[allow(unused_imports)]
use log::{error, info, warn, debug};

mod api;
#[cfg(feature = "enable_local_fs")]
mod file_localfs;
mod file_memdb;
mod file;
mod flags;
mod flip;
mod magic;
#[cfg(feature = "enable_local_fs")]
mod appender;
mod loader;
#[cfg(feature = "enable_local_fs")]
mod archive;
mod core;
mod test;

pub use flags::OpenFlags;
pub use loader::RedoLogLoader;
pub use self::core::RedoLog;
pub use api::LogWritable;

pub(crate) use api::LogLookup;

pub use file::*;