use std::ops::DerefMut;

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
use super::fs::conv_serialization;
use tokio::sync::Mutex;
use parking_lot::Mutex as PMutex;

#[derive(Debug)]
pub struct SymLink
{
    pub ino: u64,
    pub created: u64,
    pub updated: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub name: String,
    pub link: Option<String>,
}

impl SymLink
{
    pub fn new(inode: Dao<Inode>, created: u64, updated: u64) -> SymLink {
        SymLink {
            uid: inode.dentry.uid,
            gid: inode.dentry.gid,
            mode: inode.dentry.mode,
            name: inode.dentry.name.clone(),
            ino: inode.key().as_u64(),
            link: inode.link.clone(),
            created,
            updated,
        }
    }
}

#[async_trait]
impl FileApi
for SymLink
{
    fn spec(&self) -> SpecType {
        SpecType::SymLink
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn kind(&self) -> FileType {
        FileType::Symlink
    }

    fn uid(&self) -> u32 {
        self.uid
    }

    fn gid(&self) -> u32 {
        self.gid
    }

    fn mode(&self) -> u32 {
        self.mode
    }

    fn name(&self) -> String {
        self.name.clone()
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

    fn link(&self) -> Option<String> {
        self.link.clone()
    }

    fn size(&self) -> u64 {
        match &self.link {
            Some(a) => a.len() as u64,
            None => 0
        }
    }
}