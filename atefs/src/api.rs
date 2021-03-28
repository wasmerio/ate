use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use serde::*;
use super::dir::Directory;
use super::file::RegularFile;
use super::fixed::FixedFile;
use super::symlink::SymLink;
use ate::dio::Dio;
use fuse3::FileType;
use bytes::Bytes;
use ate::prelude::*;
use fuse3::{Errno, Result};

#[enum_dispatch(FileApi)]
#[derive(Debug)]
pub enum FileSpec
{
    //Custom,
    //NamedPipe,
    //CharDevice,
    //BlockDevice,
    Directory,
    RegularFile,
    SymLink,
    //Socket,
    FixedFile,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecType
{
    Directory,
    RegularFile,
    FixedFile,
    SymLink,
}

#[async_trait]
#[enum_dispatch]
pub trait FileApi
{
    fn ino(&self) -> u64;

    fn name(&self) -> String;

    fn spec(&self) -> SpecType;

    fn kind(&self) -> FileType;

    fn uid(&self) -> u32 { 0 }

    fn gid(&self) -> u32 { 0 }

    fn size(&self) -> u64 { 0 }

    fn mode(&self) -> u32 { 0 }

    fn accessed(&self) -> u64 { 0 }

    fn created(&self) -> u64 { 0 }

    fn updated(&self) -> u64 { 0 }

    async fn fallocate(&self, _chain: &Chain, _session: &AteSession, _size: u64) -> Result<()> { Ok(()) }

    async fn read(&self, _chain: &Chain, _session: &AteSession, _offset: u64, _size: u64) -> Result<Bytes> { Ok(Bytes::from(Vec::new())) }

    async fn write(&self, _chain: &Chain, _session: &AteSession, _offset: u64, _data: &[u8]) -> Result<u64> { Ok(0) }

    fn link(&self) -> Option<String> { None }

    async fn commit(&self, _chain: &Chain, _session: &AteSession) -> Result<()> { Ok(())}
}