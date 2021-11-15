#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

mod api;
#[cfg(feature = "enable_local_fs")]
mod appender;
#[cfg(feature = "enable_local_fs")]
mod archive;
mod core;
mod flags;
mod flip;
mod loader;
#[cfg(feature = "enable_local_fs")]
mod log_localfs;
mod log_memdb;
mod log_traits;
mod magic;
mod test;

pub use self::core::RedoLog;
pub use api::LogWritable;
pub use flags::OpenFlags;
pub use loader::RedoLogLoader;

pub(crate) use api::LogLookup;

pub use log_traits::*;
