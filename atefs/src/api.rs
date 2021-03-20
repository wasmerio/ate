use enum_dispatch::enum_dispatch;
use serde::*;
use super::dir::*;
use super::file::*;

#[enum_dispatch]
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

#[enum_dispatch(FileType)]
pub trait FileApi
{
}