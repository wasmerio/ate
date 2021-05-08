#[allow(unused_imports)]
use log::{error, info, warn, debug};

mod api;
mod file;
mod flags;
mod flip;
mod magic;
mod appender;
mod loader;
mod archive;
mod core;
mod test;

pub use flags::OpenFlags;
pub use loader::RedoLogLoader;
pub use self::core::RedoLog;
pub use api::LogWritable;

pub(crate) use api::LogLookup;