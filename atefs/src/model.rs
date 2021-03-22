use serde::*;
use serbia::serbia;
use ate::prelude::*;
use super::api::*;
use super::dir::Directory;
use super::file::RegularFile;
use super::fixed::FixedFile;

pub const PAGE_SIZE: usize = 2097152;
type PageBuf = [u8; PAGE_SIZE];

#[serbia]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Page {
    pub off: u64,
    #[serbia_bufsize(PAGE_SIZE)]
    pub buf: PageBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Extent {
    pub offset: u64,
    pub size: u64,
    pub pages: DaoVec<Page>,
    pub extents: DaoVec<Extent>,
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
    pub blob: DaoVec<Extent>,
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
            blob: DaoVec::default(),
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