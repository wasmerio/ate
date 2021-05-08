#[allow(unused_imports)]
use log::{error, info, warn, debug};

mod api;
mod file;
mod flags;
mod flip;
mod appender;
mod loader;
mod archive;
mod core;
mod test;

static REDO_MAGIC: u32 = u32::from_be_bytes(*b"REDO");

pub use flags::OpenFlags;
pub use loader::RedoLogLoader;
pub use self::core::RedoLog;
pub use api::LogWritable;

pub(crate) use api::LogLookup;