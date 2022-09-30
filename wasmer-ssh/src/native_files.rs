#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Clone)]
pub enum NativeFileType {
    LocalFileSystem(String),
    EmbeddedFiles,
    None,
}
