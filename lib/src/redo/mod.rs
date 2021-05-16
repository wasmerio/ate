#[allow(unused_imports)]
use log::{error, info, warn, debug};

mod api;
#[cfg(feature = "local_fs")]
mod file_localfs;
#[cfg(not(feature = "local_fs"))]
mod file_memdb;
mod flags;
mod flip;
mod magic;
#[cfg(feature = "local_fs")]
mod appender;
mod loader;
#[cfg(feature = "local_fs")]
mod archive;
mod core;
mod test;

pub use flags::OpenFlags;
pub use loader::RedoLogLoader;
pub use self::core::RedoLog;
pub use api::LogWritable;

pub(crate) use api::LogLookup;

#[cfg(feature = "local_fs")]
mod file {
    pub(crate) use super::file_localfs::*;
}

#[cfg(not(feature = "local_fs"))]
mod file {
    pub(super) use super::file_memdb::*;
}