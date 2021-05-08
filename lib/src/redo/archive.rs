#[allow(unused_imports)]
use log::{error, info, warn, debug};

use async_trait::async_trait;
use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}};
use tokio::io::Result;
use std::mem::size_of;
use std::io::SeekFrom;
use tokio::fs::File;
use tokio::sync::Mutex as Mutex;
use tokio::sync::MutexGuard;

use crate::spec::*;
use super::magic::*;

#[derive(Debug)]
pub(crate) struct LogArchive
{
    pub(crate) index: u32,
    pub(crate) path: String,
    file: Mutex<File>,
    header: Vec<u8>,
}

impl LogArchive
{
    pub async fn new(path: String, index: u32) -> Result<LogArchive>
    {
        let path = format!("{}.{}", path.clone(), index);
        let log_random_access = tokio::fs::OpenOptions::new().read(true).open(path.clone()).await?;

        let mut ret = LogArchive {
            index,
            path,
            header: Vec::new(),
            file: Mutex::new(log_random_access),
        };

        ret.header = {
            let mut guard = ret.lock_at(0).await?;
            let r = match RedoHeader::read(&mut guard).await? {
                Some(a) => Vec::from(a.inner().clone()),
                None => Vec::new(),
            };
            guard.seek(0).await?;
            r
        };

        Ok(
            ret
        )
    }

    pub async fn clone(&self) -> Result<LogArchive>
    {
        let log_back = self.file.lock().await.try_clone().await?;
        Ok(
            LogArchive {
                index: self.index,
                path: self.path.clone(),
                header: self.header.clone(),
                file: Mutex::new(log_back),                
            }
        )
    }

    pub async fn lock_at(&self, off: u64) -> Result<LogArchiveGuard<'_>>
    {
        let mut file = self.file.lock().await;
        file.seek(SeekFrom::Start(off)).await?;
        Ok(
            LogArchiveGuard {
                index: self.index,
                offset: off,
                file,
            }
        )
    }

    pub async fn len(&self) -> Result<u64> {
        Ok(self.file.lock().await.metadata().await?.len())
    }

    pub(crate) fn header(&self) -> &[u8] {
        &self.header[..]
    }
}

#[derive(Debug)]
pub(crate) struct LogArchiveGuard<'a>
{
    index: u32,
    offset: u64,
    file: MutexGuard<'a, File>,
}

impl<'a> LogArchiveGuard<'a>
{
    pub(super) fn index(&'a self) -> u32 {
        self.index
    }
}

#[async_trait]
impl<'a> LogApi
for LogArchiveGuard<'a>
{
    fn offset(&self) -> u64 {
        self.offset
    }

    async fn len(&self) -> Result<u64> {
        Ok(self.file.metadata().await?.len())
    }

    async fn seek(&mut self, off: u64) -> Result<()> {
        self.file.seek(SeekFrom::Start(off)).await?;
        self.offset = off;
        Ok(())
    }
    
    async fn read_u8(&mut self) -> Result<u8> {
        let ret = self.file.read_u8().await?;
        self.offset = self.offset + size_of::<u8>() as u64;
        Ok(ret)
    }

    async fn read_u16(&mut self) -> Result<u16> {
        let ret = self.file.read_u16().await?;
        self.offset = self.offset + size_of::<u16>() as u64;
        Ok(ret)
    }

    async fn read_u32(&mut self) -> Result<u32> {
        let ret = self.file.read_u32().await?;
        self.offset = self.offset + size_of::<u32>() as u64;
        Ok(ret)
    }

    async fn read_u64(&mut self) -> Result<u64> {
        let ret = self.file.read_u64().await?;
        self.offset = self.offset + size_of::<u64>() as u64;
        Ok(ret)
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let amt = self.file.read_exact(&mut buf[..]).await?;
        self.offset = self.offset + amt as u64;
        Ok(())
    }

    async fn write_u8(&mut self, val: u8) -> Result<()> {
        self.file.write_u8(val).await?;
        self.offset = self.offset + size_of::<u8>() as u64;
        Ok(())
    }

    async fn write_u16(&mut self, val: u16) -> Result<()> {
        self.file.write_u16(val).await?;
        self.offset = self.offset + size_of::<u16>() as u64;
        Ok(())
    }

    async fn write_u32(&mut self, val: u32) -> Result<()> {
        self.file.write_u32(val).await?;
        self.offset = self.offset + size_of::<u32>() as u64;
        Ok(())
    }

    async fn write_u64(&mut self, val: u64) -> Result<()> {
        self.file.write_u64(val).await?;
        self.offset = self.offset + size_of::<u64>() as u64;
        Ok(())
    }

    async fn write_exact(&mut self, buf: &[u8]) -> Result<()> {
        self.file.write_all(&buf[..]).await?;
        self.offset = self.offset + buf.len() as u64;
        Ok(())
    }
}