use async_trait::async_trait;
use crate::api::FileApi;
use serde::*;
use fuse3::FileType;
use super::model::*;
use ate::prelude::PrimaryKey;
use super::api::SpecType;
use ate::prelude::*;
use bytes::Bytes;
use fuse3::{Errno, Result};
use super::fs::conv_load;

#[derive(Debug)]
pub struct RegularFile
{
    pub inode: Dao<Inode>,
    pub created: u64,
    pub updated: u64,
    pub extents: Option<Vec<Dao<Extent>>>,
}

impl RegularFile
{
    pub fn new(inode: Dao<Inode>, created: u64, updated: u64) -> RegularFile {
        RegularFile {
            inode: inode,
            created,
            updated,
            extents: None,
        }
    }
}

#[async_trait]
impl FileApi
for RegularFile
{
    fn spec(&self) -> SpecType {
        SpecType::RegularFile    
    }

    fn ino(&self) -> u64 {
        self.inode.key().as_u64()
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

    fn created(&self) -> u64 {
        self.created
    }

    fn updated(&self) -> u64 {
        self.updated
    }

    fn accessed(&self) -> u64 {
        self.updated
    }

    async fn read(&self, chain: &Chain, session: &AteSession, offset: u64, size: u32) -> Result<Bytes>
    {
        let mut dio = chain.dio_for_dao(session, TransactionScope::None, &self.inode).await;

        let ret = Vec::new();
        while size > 0 {
            let repeat = false;
            for _extent in conv_load(self.inode.blob.iter(self.inode.key(), &mut dio).await)? {

            }
            if repeat == false { break; }
        }

        Ok(Bytes::from(ret))
    }
}