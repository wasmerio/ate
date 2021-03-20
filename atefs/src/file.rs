use crate::api::FileApi;
use serde::*;
use fuse3::FileType;
use super::model::*;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegularFile
{
}

impl FileApi
for RegularFile
{
    fn kind(&self, _inode: &Inode) -> FileType {
        FileType::RegularFile
    }
}