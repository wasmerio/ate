#[allow(unused_imports)]
use log::{error, info, warn, debug};

use async_trait::async_trait;
use tokio::{io::{AsyncReadExt}};
use tokio::io::Result;
use std::mem::size_of;
use tokio::io::BufStream;
use tokio::sync::Mutex as MutexAsync;

use crate::spec::*;

#[derive(Debug)]
pub(crate) struct LogArchive
{
    pub(crate) log_index: u32,
    pub(crate) log_path: String,
    pub(crate) log_random_access: MutexAsync<tokio::fs::File>,
}

#[derive(Debug)]
pub(super) struct LogArchiveReader
{
    pub(crate) log_index: u32,
    pub(crate) log_off: u64,
    pub(crate) log_stream: BufStream<tokio::fs::File>,
}

#[async_trait]
impl LogApi
for LogArchiveReader
{
    fn offset(&self) -> u64 {
        self.log_off
    }
    
    async fn read_u8(&mut self) -> Result<u8> {
        let ret = self.log_stream.read_u8().await?;
        self.log_off = self.log_off + size_of::<u8>() as u64;
        Ok(ret)
    }

    async fn read_u16(&mut self) -> Result<u16> {
        let ret = self.log_stream.read_u16().await?;
        self.log_off = self.log_off + size_of::<u16>() as u64;
        Ok(ret)
    }

    async fn read_u32(&mut self) -> Result<u32> {
        let ret = self.log_stream.read_u32().await?;
        self.log_off = self.log_off + size_of::<u32>() as u64;
        Ok(ret)
    }

    async fn read_u64(&mut self) -> Result<u64> {
        let ret = self.log_stream.read_u64().await?;
        self.log_off = self.log_off + size_of::<u64>() as u64;
        Ok(ret)
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let amt = self.log_stream.read_exact(&mut buf[..]).await?;
        self.log_off = self.log_off + amt as u64;
        Ok(())
    }

    async fn write_u8(&mut self, _: u8) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_u16(&mut self, _: u16) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_u32(&mut self, _: u32) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_u64(&mut self, _: u64) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_exact(&mut self, _: &[u8]) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }
}