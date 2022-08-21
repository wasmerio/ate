use std::path::PathBuf;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Clone)]
pub enum NativeFileType {
    LocalFileSystem(String),
    EmbeddedFiles,
    None,
}

#[derive(Debug, Clone)]
pub enum NativeFileInterface {
    LocalFileSystem(PathBuf),
    EmbeddedFiles,
    None
}
