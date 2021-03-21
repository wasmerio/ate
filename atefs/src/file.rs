use crate::api::FileApi;
use serde::*;
use fuse3::FileType;
use super::model::*;
use ate::prelude::PrimaryKey;
use super::api::SpecType;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegularFile
{
    pub key: PrimaryKey,
    pub inode: Inode
}

impl RegularFile
{
    pub fn new(key: &PrimaryKey, inode: &Inode) -> RegularFile {
        RegularFile {
            key: key.clone(),
            inode: inode.clone(),
        }
    }
}

impl FileApi
for RegularFile
{
    fn spec(&self) -> SpecType {
        SpecType::RegularFile    
    }

    fn ino(&self) -> u64 {
        self.key.as_u64()
    }

    fn kind(&self) -> FileType {
        FileType::RegularFile
    }

    fn uid(&self) -> u32 {
        self.inode.dentry.uid
    }

    fn gid(&self) -> u32 {
        self.inode.dentry.uid
    }

    fn size(&self) -> u64 {
        0
    }

    fn mode(&self) -> u32 {
        self.inode.dentry.mode
    }

    fn name(&self) -> String {
        self.inode.dentry.name.clone()
    }
}