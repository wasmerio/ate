use serde::*;
use ate::prelude::*;
use fxhash::FxHashMap;
use super::api::*;
use super::dir::Directory;
use super::file::RegularFile;
use super::fixed::FixedFile;
use super::symlink::SymLink;

pub const PAGES_PER_BUNDLE: usize = 1024;
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
    pub xattr: FxHashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Inode {
    pub kind: FileKind,
    pub dentry: Dentry,
    pub size: u64,
    pub bundles: Vec<Option<PrimaryKey>>,
    pub children: DaoVec<Inode>,
    pub link: Option<String>,
    pub xattr: DaoMap<String, String>,
}

impl Inode {
    pub fn new(name: String, mode: u32, uid: u32, gid: u32, kind: FileKind) -> Inode {
        Inode {
            kind,
            dentry: Dentry {
                name,
                mode,
                parent: None,
                uid,
                gid,
                xattr: FxHashMap::default(),
            },
            size: 0,
            bundles: Vec::default(),
            children: DaoVec::new(),
            link: None,
            xattr: DaoMap::default(),
        }
    }

    pub async fn as_file_spec(ino: u64, created: u64, updated: u64, dao: Dao<Inode>) -> FileSpec {
        match dao.kind {
            FileKind::Directory => FileSpec::Directory(Directory::new(dao, created, updated)),
            FileKind::RegularFile => FileSpec::RegularFile(RegularFile::new(dao, created, updated).await),
            FileKind::SymLink => FileSpec::SymLink(SymLink::new(dao, created, updated)),
            FileKind::FixedFile => FileSpec::FixedFile(FixedFile::new(ino, dao.dentry.name.clone(), FileKind::RegularFile).created(created).updated(updated))
        }
    }

    pub async fn as_file_spec_mut(ino: u64, created: u64, updated: u64, dao: DaoMut<Inode>) -> FileSpec {
        match dao.kind {
            FileKind::Directory => FileSpec::Directory(Directory::new_mut(dao, created, updated)),
            FileKind::RegularFile => FileSpec::RegularFile(RegularFile::new_mut(dao, created, updated).await),
            FileKind::SymLink => FileSpec::SymLink(SymLink::new_mut(dao, created, updated)),
            FileKind::FixedFile => FileSpec::FixedFile(FixedFile::new(ino, dao.dentry.name.clone(), FileKind::RegularFile).created(created).updated(updated))
        }
    }
}