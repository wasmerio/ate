#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::error::*;
use crate::event::*;

use async_trait::async_trait;
use tokio::io::Result;
pub use crate::spec::LogLookup;

#[async_trait]
pub trait LogWritable {
    /// Writes data to the redo log and returns the new offset in bytes
    async fn write(
        &mut self,
        evt: &EventData,
    ) -> std::result::Result<LogLookup, SerializationError>;
    async fn flush(&mut self) -> Result<()>;
}
