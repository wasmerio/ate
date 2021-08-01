#[allow(unused_imports)]
use tracing::{error, info, warn, debug};

use crate::event::*;
use crate::error::*;

use async_trait::async_trait;
use tokio::io::Result;

#[async_trait]
pub trait LogWritable {
    /// Writes data to the redo log and returns the new offset in bytes
    async fn write(&mut self, evt: &EventData) -> std::result::Result<LogLookup, SerializationError>;
    async fn flush(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogLookup
{
    pub(crate) index: u32,
    pub(crate) offset: u64,
}