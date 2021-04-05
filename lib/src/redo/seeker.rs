#[allow(unused_imports)]
use log::{error, info, warn, debug};

use async_trait::async_trait;
use std::{io::SeekFrom};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}};
use tokio::io::Result;
use std::mem::size_of;
use tokio::sync::Mutex as MutexAsync;
use crate::spec::*;

pub(super) struct SpecificLogLoader<'a>
{
    offset: u64,
    lock: tokio::sync::MutexGuard<'a, tokio::fs::File>,
}

impl<'a> SpecificLogLoader<'a>
{
    pub(super) async fn new(mutex: &'a MutexAsync<tokio::fs::File>, offset: u64) -> std::result::Result<SpecificLogLoader<'a>, tokio::io::Error> {
        let mut lock = mutex.lock().await;
        lock.seek(SeekFrom::Start(offset)).await?;
        Ok(SpecificLogLoader {
            offset,
            lock,
        })
    }
}

#[async_trait]
impl<'a> LogApi
for SpecificLogLoader<'a>
{
    fn offset(&self) -> u64 {
        self.offset
    }

    async fn read_u8(&mut self) -> Result<u8> {
        let ret = self.lock.read_u8().await?;
        self.offset = self.offset + size_of::<u8>() as u64;
        Ok(ret)
    }

    async fn read_u16(&mut self) -> Result<u16> {
        let ret = self.lock.read_u16().await?;
        self.offset = self.offset + size_of::<u16>() as u64;
        Ok(ret)
    }

    async fn read_u32(&mut self) -> Result<u32> {
        let ret = self.lock.read_u32().await?;
        self.offset = self.offset + size_of::<u32>() as u64;
        Ok(ret)
    }

    async fn read_u64(&mut self) -> Result<u64> {
        let ret = self.lock.read_u64().await?;
        self.offset = self.offset + size_of::<u64>() as u64;
        Ok(ret)
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let amt = self.lock.read_exact(&mut buf[..]).await?;
        self.offset = self.offset + amt as u64;
        Ok(())
    }

    async fn write_u8(&mut self, val: u8) -> Result<()> {
        self.lock.write_u8(val).await?;
        self.offset = self.offset + size_of::<u8>() as u64;
        Ok(())
    }

    async fn write_u16(&mut self, val: u16) -> Result<()> {
        self.lock.write_u16(val).await?;
        self.offset = self.offset + size_of::<u16>() as u64;
        Ok(())
    }

    async fn write_u32(&mut self, val: u32) -> Result<()> {
        self.lock.write_u32(val).await?;
        self.offset = self.offset + size_of::<u32>() as u64;
        Ok(())
    }

    async fn write_u64(&mut self, val: u64) -> Result<()> {
        self.lock.write_u64(val).await?;
        self.offset = self.offset + size_of::<u64>() as u64;
        Ok(())
    }

    async fn write_exact(&mut self, buf: &[u8]) -> Result<()> {
        self.lock.write_all(&buf[..]).await?;
        self.offset = self.offset + buf.len() as u64;
        Ok(())
    }
}