use enum_dispatch::enum_dispatch;
use serde::*;
use super::dir::Directory;
use super::file::RegularFile;
use fuse3::FileType;
use super::model::Inode;

#[enum_dispatch(FileApi)]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
}

impl Default
for FileSpec
{
    fn default() -> FileSpec {
        FileSpec::RegularFile(RegularFile{})
    }
}

#[enum_dispatch]
pub trait FileApi
{
    fn kind(&self, inode: &Inode) -> FileType;
}