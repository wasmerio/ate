#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use super::api::*;
use super::attr::*;

pub struct OpenHandle
where Self: Send + Sync
{
    pub dirty: seqlock::SeqLock<bool>,

    pub inode: u64,
    pub fh: u64,
    pub spec: FileSpec,
    pub kind: FileKind,
    pub attr: FileAttr,
    pub read_only: bool,
    
    pub children: Vec<DirectoryEntry>,
}

pub struct DirectoryEntry
where Self: Send + Sync
{
    pub inode: u64,
    pub kind: FileKind,
    pub attr: FileAttr,
    pub name: String,
    pub uid: u32,
    pub gid: u32,
}

impl OpenHandle
{
    pub fn add_child(&mut self, spec: &FileSpec, uid: u32, gid: u32) {
        self.children.push(DirectoryEntry {
            inode: spec.ino(),
            kind: spec.kind(),
            name: spec.name(),
            attr: FileAttr::new(spec, uid, gid),
            uid,
            gid
        });
    }
}