#[allow(unused_imports)]
use log::{error, info, warn, debug};
use async_trait::async_trait;

use tokio::io::{Result};

use crate::{crypto::*, redo::LogLookup};
use crate::event::*;
use crate::error::*;
use crate::loader::*;

#[async_trait]
pub trait LogFile
where Self: Sync + Send
{
    #[cfg(feature = "rotate")]
    async fn rotate(&mut self, header_bytes: Vec<u8>) -> Result<()>;

    async fn copy(&mut self) -> Result<Box<dyn LogFile>>;

    async fn write(&mut self, evt: &EventData) -> std::result::Result<LogLookup, SerializationError>;

    async fn copy_event(&mut self, from_log: &Box<dyn LogFile>, hash: AteHash) -> std::result::Result<LogLookup, LoadError>;

    async fn load(&self, hash: AteHash) -> std::result::Result<LoadData, LoadError>;

    fn move_log_file(&mut self, new_path: &String) -> Result<()>;

    async fn begin_flip(&self, header_bytes: Vec<u8>) -> Result<Box<dyn LogFile>>;

    async fn flush(&mut self) -> Result<()>;

    fn count(&self) -> usize;

    fn size(&self) -> u64;

    fn index(&self) -> u32;

    fn offset(&self) -> u64;

    fn header(&self, index: u32) -> Vec<u8>;

    fn destroy(&mut self) -> Result<()>;
}