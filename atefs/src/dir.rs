use async_trait::async_trait;
use crate::api::FileApi;
use ate::header::PrimaryKey;
use serde::*;
use fuse3::FileType;
use super::model::*;
use super::api::SpecType;
use ate::prelude::*;

#[derive(Debug)]
pub struct Directory
{
    pub inode: Dao<Inode>,
    pub created: u64,
    pub updated: u64,
}

impl Directory
{
    pub fn new(inode: Dao<Inode>, created: u64, updated: u64) -> Directory {
        Directory {
            inode: inode,
            created,
            updated,
        }
    }
}

#[async_trait]
impl FileApi
for Directory
{
    fn spec(&self) -> SpecType {
        SpecType::Directory    
    }

    fn ino(&self) -> u64 {
        self.inode.key().as_u64()
    }

    fn kind(&self) -> FileType {
        FileType::Directory
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

    fn created(&self) -> u64 {
        self.created
    }

    fn updated(&self) -> u64 {
        self.updated
    }

    fn accessed(&self) -> u64 {
        self.updated
    }
}