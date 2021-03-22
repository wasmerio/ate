use serde::*;
use serbia::serbia;
use ate::prelude::*;
use super::api::*;
use super::dir::Directory;
use super::file::RegularFile;
use super::fixed::FixedFile;

pub const BUNDLE_SIZE: usize = 1024;
pub const PAGE_SIZE: usize = 131072;

/// Represents a block of data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Page {
    pub buf: Vec<u8>,
}

/// Represents a bundle of 1024 pages
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageBundle {
    pub pages: Vec<Option<PrimaryKey>>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Dentry {
    pub parent: Option<u64>,
    pub name: String,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Inode {
    pub spec_type: SpecType,
    pub dentry: Dentry,
    pub size: u64,
    pub bundles: Vec<Option<PrimaryKey>>,
    pub children: DaoVec<Inode>,
}

impl Inode {
    pub fn new(name: String, mode: u32, uid: u32, gid: u32, spec_type: SpecType) -> Inode {
        Inode {
            spec_type,
            dentry: Dentry {
                name,
                mode,
                parent: None,
                uid,
                gid,
            },
            size: 0,
            bundles: Vec::default(),
            children: DaoVec::default(),
        }
    }

    pub fn as_file_spec(ino: u64, created: u64, updated: u64, dao: Dao<Inode>) -> FileSpec {
        match dao.spec_type {
            SpecType::Directory => FileSpec::Directory(Directory::new(dao, created, updated)),
            SpecType::RegularFile => FileSpec::RegularFile(RegularFile::new(dao, created, updated)),
            _ => FileSpec::FixedFile(FixedFile::new(ino, dao.dentry.name.clone(), fuse3::FileType::RegularFile).created(created).updated(updated))
        }
    }
}