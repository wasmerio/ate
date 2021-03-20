use crate::api::FileApi;
use serde::*;
use fuse3::FileType;
use super::model::*;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Directory
{
}

impl FileApi
for Directory
{
    fn kind(&self, _inode: &Inode) -> FileType {
        FileType::Directory
    }
}