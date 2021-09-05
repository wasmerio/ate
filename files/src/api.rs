use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use serde::*;
use super::dir::Directory;
use super::file::RegularFile;
use super::fixed::FixedFile;
use super::symlink::SymLink;
use bytes::Bytes;
use fxhash::FxHashMap;

use crate::error::*;

#[enum_dispatch(FileApi)]
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileKind
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

    fn kind(&self) -> FileKind;

    fn uid(&self) -> u32 { 0 }

    fn gid(&self) -> u32 { 0 }

    fn size(&self) -> u64 { 0 }

    fn mode(&self) -> u32 { 0 }

    fn accessed(&self) -> u64 { 0 }

    fn created(&self) -> u64 { 0 }

    fn updated(&self) -> u64 { 0 }

    async fn fallocate(&self, _size: u64) -> Result<()> { Ok(()) }

    async fn read(&self, _offset: u64, _size: u64) -> Result<Bytes> { Ok(Bytes::from(Vec::new())) }

    async fn write(&self, _offset: u64, _data: &[u8]) -> Result<u64> { Ok(0) }

    fn link(&self) -> Option<String> { None }

    async fn commit(&self) -> Result<()> { Ok(())}

    async fn set_xattr(&mut self, _name: &str, _value: &str) -> Result<()> { Err(FileSystemErrorKind::NotImplemented.into()) }

    async fn remove_xattr(&mut self, _name: &str) -> Result<bool> { Ok(false) }

    async fn get_xattr(&self, _name: &str) -> Result<Option<String>> { Ok(None) }

    async fn list_xattr(&self) -> Result<FxHashMap<String, String>> { Ok(FxHashMap::default()) }
}