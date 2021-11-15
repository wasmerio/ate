use super::api::FileKind;
use super::model::*;
use crate::api::FileApi;
use async_trait::async_trait;
use error_chain::bail;
use fxhash::FxHashMap;
use std::ops::Deref;

use ate::prelude::*;

use crate::error::*;

#[derive(Debug)]
pub struct Directory {
    pub state: DirectoryState,
    pub key: PrimaryKey,
    pub created: u64,
    pub updated: u64,
}

#[derive(Debug)]
pub enum DirectoryState {
    Immutable { inode: Dao<Inode> },
    Mutable { inode: DaoMut<Inode> },
}

impl Directory {
    pub fn new(inode: Dao<Inode>, created: u64, updated: u64) -> Directory {
        Directory {
            key: inode.key().clone(),
            state: DirectoryState::Immutable { inode },
            created,
            updated,
        }
    }

    pub fn new_mut(inode: DaoMut<Inode>, created: u64, updated: u64) -> Directory {
        Directory {
            key: inode.key().clone(),
            state: DirectoryState::Mutable { inode },
            created,
            updated,
        }
    }
}

#[async_trait]
impl FileApi for Directory {
    fn kind(&self) -> FileKind {
        FileKind::Directory
    }

    fn ino(&self) -> u64 {
        self.key.as_u64()
    }

    fn uid(&self) -> u32 {
        match &self.state {
            DirectoryState::Immutable { inode } => inode.dentry.uid,
            DirectoryState::Mutable { inode } => inode.dentry.uid,
        }
    }

    fn gid(&self) -> u32 {
        match &self.state {
            DirectoryState::Immutable { inode } => inode.dentry.gid,
            DirectoryState::Mutable { inode } => inode.dentry.gid,
        }
    }

    fn size(&self) -> u64 {
        0
    }

    fn mode(&self) -> u32 {
        match &self.state {
            DirectoryState::Immutable { inode } => inode.dentry.mode,
            DirectoryState::Mutable { inode } => inode.dentry.mode,
        }
    }

    fn name(&self) -> String {
        match &self.state {
            DirectoryState::Immutable { inode } => inode.dentry.name.clone(),
            DirectoryState::Mutable { inode } => inode.dentry.name.clone(),
        }
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

    async fn set_xattr(&mut self, name: &str, value: &str) -> Result<()> {
        match &mut self.state {
            DirectoryState::Immutable { inode: _ } => bail!(FileSystemErrorKind::NoAccess),
            DirectoryState::Mutable { inode } => {
                inode
                    .as_mut()
                    .xattr
                    .insert(name.to_string(), value.to_string())
                    .await?
            }
        };
        Ok(())
    }

    async fn remove_xattr(&mut self, name: &str) -> Result<bool> {
        let name = name.to_string();
        let ret = match &mut self.state {
            DirectoryState::Immutable { inode: _ } => bail!(FileSystemErrorKind::NoAccess),
            DirectoryState::Mutable { inode } => inode.as_mut().xattr.delete(&name).await?,
        };
        Ok(ret)
    }

    async fn get_xattr(&self, name: &str) -> Result<Option<String>> {
        let name = name.to_string();
        let ret = match &self.state {
            DirectoryState::Immutable { inode } => {
                inode.xattr.get(&name).await?.map(|a| a.deref().clone())
            }
            DirectoryState::Mutable { inode } => {
                inode.xattr.get(&name).await?.map(|a| a.deref().clone())
            }
        };
        Ok(ret)
    }

    async fn list_xattr(&self) -> Result<FxHashMap<String, String>> {
        let mut ret = FxHashMap::default();
        match &self.state {
            DirectoryState::Mutable { inode } => {
                for (k, v) in inode.xattr.iter().await? {
                    ret.insert(k, v.deref().clone());
                }
            }
            DirectoryState::Immutable { inode } => {
                for (k, v) in inode.xattr.iter().await? {
                    ret.insert(k, v.deref().clone());
                }
            }
        };
        Ok(ret)
    }
}
