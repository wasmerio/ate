use enum_dispatch::enum_dispatch;
use serde::*;
use super::dir::Directory;
use super::file::RegularFile;
use super::fixed::FixedFile;
use fuse3::FileType;

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
    //Symlink,
    //Socket,
    FixedFile,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecType
{
    Directory,
    RegularFile,
    FixedFile,
}

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
}