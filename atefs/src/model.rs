use serde::*;
use serbia::serbia;
use ate::prelude::*;
use super::api::*;

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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Inode {
    pub spec: FileSpec,
    pub dentry: Dentry,
    pub blob: DaoVec<Extent>,
    pub children: DaoVec<Inode>,
}

impl Inode {
    pub fn new(name: String, mode: u32, spec: FileSpec) -> Inode {
        Inode {
            spec,
            dentry: Dentry {
                name,
                mode,
                parent: None,
            },
            blob: DaoVec::default(),
            children: DaoVec::default(),
        }
    }
}