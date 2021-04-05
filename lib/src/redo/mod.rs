#[allow(unused_imports)]
use log::{error, info, warn, debug};

mod api;
mod file;
mod flags;
mod flip;
mod loader;
mod reader;
mod core;
mod seeker;
mod test;

static REDO_MAGIC: &'static [u8; 4] = b"REDO";

pub use flags::OpenFlags;
pub use loader::RedoLogLoader;
pub use self::core::RedoLog;
pub use api::LogWritable;