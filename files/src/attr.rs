use super::prelude::*;

#[derive(Debug, Clone)]
pub struct FileAttr
{
    pub ino: u64,
    pub size: u64,
    pub blksize: u32,
    pub accessed: u64,
    pub updated: u64,
    pub created: u64,
    pub kind: FileKind,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

impl FileAttr
{
    pub fn new(spec: &FileSpec, uid: u32, gid: u32) -> FileAttr {
        let size = spec.size();
        let blksize = PAGE_SIZE as u64;
    
        FileAttr {
            ino: spec.ino(),
            size,
            accessed: spec.accessed(),
            updated: spec.updated(),
            created: spec.created(),
            kind: spec.kind(),
            mode: spec.mode(),
            uid,
            gid,
            blksize: blksize as u32,
        }
    }
}

impl FileAccessor
{
    pub fn spec_as_attr_reverse(&self, spec: &FileSpec, req: &RequestContext) -> FileAttr {
        let uid = self.reverse_uid(spec.uid(), req);
        let gid = self.reverse_gid(spec.gid(), req);
        FileAttr::new(spec, uid, gid)
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SetAttr {
    /// set file or directory mode.
    pub mode: Option<u32>,
    /// set file or directory uid.
    pub uid: Option<u32>,
    /// set file or directory gid.
    pub gid: Option<u32>,
    /// set file or directory size.
    pub size: Option<u64>,
    /// the lock_owner argument.
    pub lock_owner: Option<u64>,
    /// set file or directory atime.
    pub accessed: Option<u64>,
    /// set file or directory mtime.
    pub updated: Option<u64>,
    /// set file or directory ctime.
    pub created: Option<u64>,
}