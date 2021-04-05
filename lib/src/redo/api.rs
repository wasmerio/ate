#[allow(unused_imports)]
use log::{error, info, warn, debug};

use crate::event::*;
use crate::error::*;

use async_trait::async_trait;
use tokio::io::Result;

#[async_trait]
pub trait LogWritable {
    async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError>;
    async fn flush(&mut self) -> Result<()>;
}