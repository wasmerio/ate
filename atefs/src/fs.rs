#![allow(unused_imports)]
use log::{info, error, debug};
use serde::__private::de::IdentifierDeserializer;

use std::{collections::BTreeMap, ops::Deref};
use std::ffi::{OsStr, OsString};
use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::vec::IntoIter;
use parking_lot::Mutex;

use ate::dio::Dio;
use ate::dio::Dao;
use ate::error::*;
use ate::chain::*;
use ate::session::Session as AteSession;
use ate::header::PrimaryKey;
use super::dir::Directory;
use super::file::RegularFile;
use super::model::*;
use super::api::*;

use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use futures_util::stream;
use futures_util::stream::{Empty, Iter};
use futures_util::StreamExt;
use tokio::sync::RwLock;
use fxhash::FxHashMap;

use fuse3::raw::prelude::*;
use fuse3::{Errno, Result};

#[allow(dead_code)]
const TTL: Duration = Duration::from_secs(1);

pub struct AteFS
where Self: Send + Sync
{
    pub chain: Chain,
    pub session: AteSession,
    pub open_handles: Mutex<FxHashMap<u64, OpenHandle>>
}

pub struct OpenHandle
where Self: Send + Sync
{
    pub inode: u64,
    pub fh: u64,
    pub attr: FileAttr,
    pub children: Vec<DirectoryEntry>,
    pub children_plus: Vec<DirectoryEntryPlus>,
}

impl OpenHandle
{
    fn add_child(&mut self, inode: u64, kind: FileType, name: String, mode: u32) {
        let attr = FileAttr {
            ino: inode,
            generation: 0,
            size: 0,
            blocks: 0,
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            kind: kind,
            perm: fuse3::perm_from_mode_and_kind(kind, mode),
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
        };

        self.children.push(DirectoryEntry {
            inode,
            kind,
            name: OsString::from(name.clone()),
        });
        self.children_plus.push(DirectoryEntryPlus {
            inode,
            kind,
            name: OsString::from(name.clone()),
            generation: 0,
            attr,
            entry_ttl: TTL,
            attr_ttl: TTL,
        });
    }
}

impl Inode
{
    pub fn attr(&self, id: u64) -> FileAttr {
        FileAttr {
            ino: id,
            generation: 0,
            size: 0,
            blocks: 0,
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            kind: self.spec.kind(self),
            perm: fuse3::perm_from_mode_and_kind(self.spec.kind(self), self.dentry.mode),
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
        }
    }
}

fn conv_load<T>(r: std::result::Result<T, LoadError>) -> std::result::Result<T, Errno> {
    conv(match r {
        Ok(a) => Ok(a),
        Err(err) => Err(AteError::LoadError(err)),
    })
}

fn conv_io<T>(r: std::result::Result<T, tokio::io::Error>) -> std::result::Result<T, Errno> {
    conv(match r {
        Ok(a) => Ok(a),
        Err(err) => Err(AteError::IO(err)),
    })
}

fn conv_serialization<T>(r: std::result::Result<T, SerializationError>) -> std::result::Result<T, Errno> {
    conv(match r {
        Ok(a) => Ok(a),
        Err(err) => Err(AteError::SerializationError(err)),
    })
}

fn conv<T>(r: std::result::Result<T, AteError>) -> std::result::Result<T, Errno> {
    match r {
        Ok(a) => Ok(a),
        Err(err) => {
            debug!("atefs::error {}", err);
            match err {
                AteError::LoadError(LoadError::NotFound(_)) => Err(libc::ENOSYS.into()),
                _ => Err(libc::ENOSYS.into())
            }
        }
    }
}

impl AteFS
{
    pub fn new(chain: Chain) -> AteFS {
        let session = AteSession::default();
        AteFS {
            chain,
            session,
            open_handles: Mutex::new(FxHashMap::default()),
        }
    }

    pub async fn load(&self, inode: u64) -> Result<Inode> {
        let mut dio = self.chain.dio(&self.session).await;
        let dao = conv_load(dio.load::<Inode>(&PrimaryKey::from(inode)).await)?;
        Ok(dao.clone())
    }

    async fn create_open_handle(&self, inode: u64) -> Result<OpenHandle>
    {
        let mut dio = self.chain.dio(&self.session).await;
        let data = conv_load(dio.load::<Inode>(&PrimaryKey::from(inode)).await)?;
        
        let key = PrimaryKey::from(inode);
        let mut open = OpenHandle {
            inode,
            fh: fastrand::u64(..),
            children: Vec::new(),
            children_plus: Vec::new(),
            attr: data.attr(inode),
        };
        
        open.add_child(inode, FileType::Directory, ".".to_string(), data.dentry.mode);
        open.add_child(inode, FileType::Directory, "..".to_string(), data.dentry.mode);
        for child in conv_load(data.children.iter(&key, &mut dio).await)? {
            open.add_child(child.key().as_u64(), child.spec.kind(child.deref()), child.dentry.name.clone(), child.dentry.mode);
        }

        Ok(open)
    }
}

#[async_trait]
impl Filesystem
for AteFS
{
    type DirEntryStream = Iter<IntoIter<Result<DirectoryEntry>>>;
    type DirEntryPlusStream = Iter<IntoIter<Result<DirectoryEntryPlus>>>;

    async fn init(&self, _req: Request) -> Result<()>
    {
        // Attempt to load the root node, if it does not exist then create it
        //let mut dio = self.chain.dio_ext(&self.session, Scope::Full).await;
        let mut dio = self.chain.dio(&self.session).await;
        if let Err(LoadError::NotFound(_)) = dio.load::<Inode>(&PrimaryKey::from(1)).await {
            info!("atefs::creating-root-node");

            let root = Inode::new("/".to_string(), 0o755, FileSpec::Directory(Directory::default()));
            match dio.store_ext(root, None, Some(PrimaryKey::from(1))) {
                Ok(_) => { },
                Err(err) => {
                    debug!("atefs::error {}", err);        
                }
            }     
       };
        info!("atefs::init");

        Ok(())
    }

    async fn destroy(&self, _req: Request) {
        info!("atefs::destroy");
    }

    async fn readdirplus(
        &self,
        _req: Request,
        parent: u64,
        fh: u64,
        offset: u64,
        _lock_owner: u64,
    ) -> Result<ReplyDirectoryPlus<Self::DirEntryPlusStream>> {        
        debug!("atefs::readdirplus id={} offset={}", parent, offset);

        if fh == 0 {
            let open = self.create_open_handle(parent).await?;
            let entries = open.children_plus.iter().skip(offset as usize).map(|a| Ok(a.clone())).collect::<Vec<_>>();
            return Ok(ReplyDirectoryPlus {
                entries: stream::iter(entries.into_iter())
            });
        }

        let lock = self.open_handles.lock();
        if let Some(open) = lock.get(&fh) {
            let entries = open.children_plus.iter().skip(offset as usize).map(|a| Ok(a.clone())).collect::<Vec<_>>();
            Ok(ReplyDirectoryPlus {
                entries: stream::iter(entries.into_iter())
            })
        } else {
            Err(libc::ENOSYS.into())
        }
    }

    async fn readdir(
        &self,
        _req: Request,
        parent: u64,
        fh: u64,
        offset: i64,
    ) -> Result<ReplyDirectory<Self::DirEntryStream>> {
        debug!("atefs::readdir parent={}", parent);

        if fh == 0 {
            let open = self.create_open_handle(parent).await?;
            let entries = open.children.iter().skip(offset as usize).map(|a| Ok(a.clone())).collect::<Vec<_>>();
            return Ok(ReplyDirectory {
                entries: stream::iter(entries.into_iter())
            });
        }

        let lock = self.open_handles.lock();
        if let Some(open) = lock.get(&fh) {
            let entries = open.children.iter().skip(offset as usize).map(|a| Ok(a.clone())).collect::<Vec<_>>();
            Ok(ReplyDirectory {
                entries: stream::iter(entries.into_iter())
            })
        } else {
            Err(libc::ENOSYS.into())
        }
    }

    async fn getattr(
        &self,
        _req: Request,
        inode: u64,
        fh: Option<u64>,
        _flags: u32,
    ) -> Result<ReplyAttr> {
        debug!("atefs::getattr inode={}", inode);

        if let Some(fh) = fh {
            let lock = self.open_handles.lock();
            if let Some(open) = lock.get(&fh) {
                return Ok(ReplyAttr {
                    ttl: TTL,
                    attr: open.attr,
                })
            }
        }

        let dao = self.load(inode).await?;
        Ok(ReplyAttr {
            ttl: TTL,
            attr: dao.attr(inode),
        })
    }

    async fn opendir(&self, _req: Request, inode: u64, _flags: u32) -> Result<ReplyOpen> {
        debug!("atefs::opendir inode={}", inode);

        let open = self.create_open_handle(inode).await?;

        if open.attr.kind != FileType::Directory {
            debug!("atefs::opendir not-a-directory");
            return Err(libc::ENOTDIR.into());
        }

        let fh = open.fh;
        self.open_handles.lock().insert(open.fh, open);

        Ok(ReplyOpen { fh, flags: 0 })
    }

    async fn releasedir(&self, _req: Request, inode: u64, fh: u64, _flags: u32) -> Result<()> {
        debug!("atefs::releasedir inode={}", inode);
        self.open_handles.lock().remove(&fh);
        Ok(())
    }

    async fn lookup(&self, _req: Request, parent: u64, name: &OsStr) -> Result<ReplyEntry> {
        let open = self.create_open_handle(parent).await?;

        if open.attr.kind != FileType::Directory {
            debug!("atefs::lookup parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(entry) = open.children_plus.iter().filter(|c| *c.name == *name).next() {
            debug!("atefs::lookup parent={} name={}: found", parent, name.to_str().unwrap());
            return Ok(ReplyEntry {
                ttl: TTL,
                attr: entry.attr,
                generation: 0,
            });
        }

        debug!("atefs::lookup parent={} name={}: not found", parent, name.to_str().unwrap());
        Err(libc::ENOENT.into())
    }

    async fn forget(&self, _req: Request, _inode: u64, _nlookup: u64) {}

    async fn fsync(&self, _req: Request, inode: u64, _fh: u64, _datasync: bool) -> Result<()> {
        debug!("atefs::fsync inode={}", inode);
        Ok(())
    }

    async fn flush(&self, _req: Request, inode: u64, _fh: u64, _lock_owner: u64) -> Result<()> {
        debug!("atefs::flush inode={}", inode);
        conv_io(self.chain.flush().await)?;
        Ok(())
    }

    async fn access(&self, _req: Request, inode: u64, _mask: u32) -> Result<()> {
        debug!("atefs::access inode={}", inode);
        Ok(())
    }

    async fn mkdir(
        &self,
        _req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
    ) -> Result<ReplyEntry> {
        debug!("atefs::mkdir parent={}", parent);

        let mut dio = self.chain.dio(&self.session).await;
        let data = conv_load(dio.load::<Inode>(&PrimaryKey::from(parent)).await)?;

        if data.spec.kind(data.deref()) != FileType::Directory {
            return Err(libc::ENOTDIR.into());
        }

        let child = Inode::new(name.to_str().unwrap().to_string(), mode, FileSpec::Directory(Directory::default()));

        let key = PrimaryKey::from(parent);
        let child = conv_serialization(data.children.push(&mut dio, &key, child))?;

        Ok(ReplyEntry {
            ttl: TTL,
            attr: child.attr(child.key().as_u64()),
            generation: 0,
        })
    }

    async fn rmdir(&self, _req: Request, parent: u64, name: &OsStr) -> Result<()> {
        debug!("atefs::rmdir parent={}", parent);

        let open = self.create_open_handle(parent).await?;

        if open.attr.kind != FileType::Directory {
            debug!("atefs::rmdir parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(entry) = open.children_plus.iter().filter(|c| *c.name == *name).next() {
            debug!("atefs::rmdir parent={} name={}: found", parent, name.to_str().unwrap());

            let mut dio = self.chain.dio(&self.session).await;
            let data = conv_load(dio.load::<Inode>(&PrimaryKey::from(entry.inode)).await)?;

            if let Some(_) = conv_load(data.children.iter(data.key(), &mut dio).await)?.next() {
                return Err(Errno::from(libc::ENOTEMPTY));
            }

            return Ok(())
        }

        debug!("atefs::rmdir parent={} name={}: not found", parent, name.to_str().unwrap());
        Err(libc::ENOENT.into())
    }
}

/*
#[derive(Debug, Clone)]
enum Entry {
    Dir(Arc<RwLock<Dir>>),
    File(Arc<RwLock<File>>),
}

impl Entry {
    async fn attr(&self) -> FileAttr {
        match self {
            Entry::Dir(dir) => {
                let dir = dir.read().await;

                FileAttr {
                    ino: dir.inode,
                    generation: 0,
                    size: 0,
                    blocks: 0,
                    atime: SystemTime::UNIX_EPOCH,
                    mtime: SystemTime::UNIX_EPOCH,
                    ctime: SystemTime::UNIX_EPOCH,
                    kind: FileType::Directory,
                    perm: fuse3::perm_from_mode_and_kind(FileType::Directory, dir.mode),
                    nlink: 0,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    blksize: 0,
                }
            }

            Entry::File(file) => {
                let file = file.read().await;

                FileAttr {
                    ino: file.inode,
                    generation: 0,
                    size: file.content.len() as _,
                    blocks: 0,
                    atime: SystemTime::UNIX_EPOCH,
                    mtime: SystemTime::UNIX_EPOCH,
                    ctime: SystemTime::UNIX_EPOCH,
                    kind: FileType::RegularFile,
                    perm: fuse3::perm_from_mode_and_kind(FileType::RegularFile, file.mode),
                    nlink: 0,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    blksize: 0,
                }
            }
        }
    }

    async fn set_attr(&self, set_attr: SetAttr) -> FileAttr {
        match self {
            Entry::Dir(dir) => {
                let mut dir = dir.write().await;

                if let Some(mode) = set_attr.mode {
                    dir.mode = mode;
                }
            }

            Entry::File(file) => {
                let mut file = file.write().await;

                if let Some(size) = set_attr.size {
                    file.content.truncate(size as _);
                }

                if let Some(mode) = set_attr.mode {
                    file.mode = mode;
                }
            }
        }

        self.attr().await
    }

    fn is_dir(&self) -> bool {
        matches!(self, Entry::Dir(_))
    }

    #[allow(dead_code)]
    fn is_file(&self) -> bool {
        !self.is_dir()
    }

    async fn inode(&self) -> u64 {
        match self {
            Entry::Dir(dir) => {
                let dir = dir.read().await;

                dir.inode
            }

            Entry::File(file) => {
                let file = file.read().await;

                file.inode
            }
        }
    }

    fn kind(&self) -> FileType {
        if self.is_dir() {
            FileType::Directory
        } else {
            FileType::RegularFile
        }
    }
}

#[derive(Debug)]
struct Dir {
    inode: u64,
    parent: u64,
    name: OsString,
    children: BTreeMap<OsString, Entry>,
    mode: u32,
}

#[derive(Debug)]
struct File {
    inode: u64,
    parent: u64,
    name: OsString,
    content: Vec<u8>,
    mode: u32,
}

#[derive(Debug)]
struct InnerFs {
    inode_map: BTreeMap<u64, Entry>,
    inode_gen: AtomicU64,
}

#[derive(Debug)]
pub struct AteFS(Chain);

impl Default for AteFS {
    fn default() -> Self {
        let root = Entry::Dir(Arc::new(RwLock::new(Dir {
            inode: 1,
            parent: 1,
            name: OsString::from("/"),
            children: BTreeMap::new(),
            mode: 0o755,
        })));

        let mut inode_map = BTreeMap::new();

        inode_map.insert(1, root);

        Self(RwLock::new(InnerFs {
            inode_map,
            inode_gen: AtomicU64::new(2),
        }))
    }
}

#[async_trait]
impl Filesystem for AteFS {
    type DirEntryStream = Iter<std::iter::Skip<IntoIter<Result<DirectoryEntry>>>>;
    type DirEntryPlusStream = Iter<IntoIter<Result<DirectoryEntryPlus>>>;

    async fn init(&self, _req: Request) -> Result<()> {
        Ok(())
    }

    async fn destroy(&self, _req: Request) {}

    async fn lookup(&self, _req: Request, parent: u64, name: &OsStr) -> Result<ReplyEntry> {
        debug!("atefs::lookup parent={}", parent);
        let inner = self.0.read().await;

        let entry = inner
            .inode_map
            .get(&parent)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::Dir(dir) = entry {
            let dir = dir.read().await;

            let attr = dir
                .children
                .get(name)
                .ok_or_else(|| Errno::from(libc::ENOENT))?
                .attr()
                .await;

            Ok(ReplyEntry {
                ttl: TTL,
                attr,
                generation: 0,
            })
        } else {
            Err(libc::ENOTDIR.into())
        }
    }

    async fn forget(&self, _req: Request, _inode: u64, _nlookup: u64) {}

    async fn getattr(
        &self,
        _req: Request,
        inode: u64,
        _fh: Option<u64>,
        _flags: u32,
    ) -> Result<ReplyAttr> {
        debug!("atefs::getattr inode={}", inode);
        Ok(ReplyAttr {
            ttl: TTL,
            attr: self
                .0
                .read()
                .await
                .inode_map
                .get(&inode)
                .ok_or_else(|| Errno::from(libc::ENOENT))?
                .attr()
                .await,
        })
    }

    async fn setattr(
        &self,
        _req: Request,
        inode: u64,
        _fh: Option<u64>,
        set_attr: SetAttr,
    ) -> Result<ReplyAttr> {
        debug!("atefs::setattr inode={}", inode);
        Ok(ReplyAttr {
            ttl: TTL,
            attr: self
                .0
                .read()
                .await
                .inode_map
                .get(&inode)
                .ok_or_else(|| Errno::from(libc::ENOENT))?
                .set_attr(set_attr)
                .await,
        })
    }

    async fn mkdir(
        &self,
        _req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
    ) -> Result<ReplyEntry> {
        debug!("atefs::mkdir parent={}", parent);
        let mut inner = self.0.write().await;

        let entry = inner
            .inode_map
            .get(&parent)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::Dir(dir) = entry {
            let mut dir = dir.write().await;

            if dir.children.get(name).is_some() {
                return Err(libc::EEXIST.into());
            }

            let new_inode = inner.inode_gen.fetch_add(1, Ordering::Relaxed);

            let entry = Entry::Dir(Arc::new(RwLock::new(Dir {
                inode: new_inode,
                parent,
                name: name.to_owned(),
                children: BTreeMap::new(),
                mode,
            })));

            let attr = entry.attr().await;

            dir.children.insert(name.to_os_string(), entry.clone());

            drop(dir); // fix inner can't borrow as mut next line

            inner.inode_map.insert(new_inode, entry);

            Ok(ReplyEntry {
                ttl: TTL,
                attr,
                generation: 0,
            })
        } else {
            Err(libc::ENOTDIR.into())
        }
    }

    async fn unlink(&self, _req: Request, parent: u64, name: &OsStr) -> Result<()> {
        debug!("atefs::unlink parent={}", parent);
        let mut inner = self.0.write().await;

        let entry = inner
            .inode_map
            .get(&parent)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::Dir(dir) = entry {
            let mut dir = dir.write().await;

            if dir
                .children
                .get(name)
                .ok_or_else(|| Errno::from(libc::ENOENT))?
                .is_dir()
            {
                return Err(libc::EISDIR.into());
            }

            let inode = dir.children.remove(name).unwrap().inode().await;

            drop(dir); // fix inner can't borrow as mut next line

            inner.inode_map.remove(&inode);

            Ok(())
        } else {
            Err(libc::ENOTDIR.into())
        }
    }

    async fn rmdir(&self, _req: Request, parent: u64, name: &OsStr) -> Result<()> {
        debug!("atefs::rmdir parent={}", parent);
        let mut inner = self.0.write().await;

        let entry = inner
            .inode_map
            .get(&parent)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::Dir(dir) = entry {
            let mut dir = dir.write().await;

            if let Entry::Dir(child_dir) =
                dir.children.get(name).ok_or_else(Errno::new_not_exist)?
            {
                if !child_dir.read().await.children.is_empty() {
                    return Err(Errno::from(libc::ENOTEMPTY));
                }
            } else {
                return Err(Errno::new_is_not_dir());
            }

            let inode = dir.children.remove(name).unwrap().inode().await;

            drop(dir); // fix inner can't borrow as mut next line

            inner.inode_map.remove(&inode);

            Ok(())
        } else {
            Err(libc::ENOTDIR.into())
        }
    }

    async fn rename(
        &self,
        _req: Request,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: &OsStr,
    ) -> Result<()> {
        debug!("atefs::rename parent={}", parent);
        let inner = self.0.read().await;

        let parent_entry = inner
            .inode_map
            .get(&parent)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::Dir(parent_dir) = parent_entry {
            let mut parent_dir = parent_dir.write().await;

            if parent == new_parent {
                let entry = parent_dir
                    .children
                    .remove(name)
                    .ok_or_else(|| Errno::from(libc::ENOENT))?;
                parent_dir.children.insert(new_name.to_os_string(), entry);

                return Ok(());
            }

            let new_parent_entry = inner
                .inode_map
                .get(&new_parent)
                .ok_or_else(|| Errno::from(libc::ENOENT))?;

            if let Entry::Dir(new_parent_dir) = new_parent_entry {
                let mut new_parent_dir = new_parent_dir.write().await;

                let entry = parent_dir
                    .children
                    .remove(name)
                    .ok_or_else(|| Errno::from(libc::ENOENT))?;
                new_parent_dir
                    .children
                    .insert(new_name.to_os_string(), entry);

                return Ok(());
            }
        }

        Err(libc::ENOTDIR.into())
    }

    async fn open(&self, _req: Request, inode: u64, _flags: u32) -> Result<ReplyOpen> {
        debug!("atefs::open inode={}", inode);
        let inner = self.0.read().await;

        let entry = inner
            .inode_map
            .get(&inode)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if matches!(entry, Entry::File(_)) {
            Ok(ReplyOpen { fh: 0, flags: 0 })
        } else {
            Err(libc::EISDIR.into())
        }
    }

    async fn read(
        &self,
        _req: Request,
        inode: u64,
        _fh: u64,
        offset: u64,
        size: u32,
    ) -> Result<ReplyData> {
        debug!("atefs::read inode={}", inode);
        let inner = self.0.read().await;

        let entry = inner
            .inode_map
            .get(&inode)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::File(file) = entry {
            let file = file.read().await;

            let mut cursor = Cursor::new(&file.content);
            cursor.set_position(offset);

            let size = cursor.remaining().min(size as _);

            let mut data = BytesMut::with_capacity(size);
            // safety
            unsafe {
                data.set_len(size);
            }

            cursor.read_exact(&mut data).unwrap();

            Ok(ReplyData { data: data.into() })
        } else {
            Err(libc::EISDIR.into())
        }
    }

    async fn write(
        &self,
        _req: Request,
        inode: u64,
        _fh: u64,
        offset: u64,
        mut data: &[u8],
        _flags: u32,
    ) -> Result<ReplyWrite> {
        debug!("atefs::write inode={}", inode);
        let inner = self.0.read().await;

        let entry = inner
            .inode_map
            .get(&inode)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::File(file) = entry {
            let mut file = file.write().await;

            if file.content.len() > offset as _ {
                let mut content = &mut file.content[offset as _..];

                if content.len() > data.len() {
                    io::copy(&mut data, &mut content).unwrap();

                    return Ok(ReplyWrite {
                        written: data.len() as _,
                    });
                }

                let n = io::copy(&mut (&data[..content.len()]), &mut content).unwrap();

                file.content.extend_from_slice(&data[n as _..]);

                Ok(ReplyWrite {
                    written: data.len() as _,
                })
            } else {
                file.content.resize(offset as _, 0);

                file.content.extend_from_slice(&data);

                Ok(ReplyWrite {
                    written: data.len() as _,
                })
            }
        } else {
            Err(libc::EISDIR.into())
        }
    }

    async fn release(
        &self,
        _req: Request,
        inode: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> Result<()> {
        debug!("atefs::release inode={}", inode);
        Ok(())
    }

    async fn fsync(&self, _req: Request, inode: u64, _fh: u64, _datasync: bool) -> Result<()> {
        debug!("atefs::fsync inode={}", inode);
        Ok(())
    }

    async fn flush(&self, _req: Request, inode: u64, _fh: u64, _lock_owner: u64) -> Result<()> {
        debug!("atefs::flush inode={}", inode);
        Ok(())
    }

    async fn access(&self, _req: Request, inode: u64, _mask: u32) -> Result<()> {
        debug!("atefs::access inode={}", inode);
        Ok(())
    }

    async fn create(
        &self,
        _req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> Result<ReplyCreated> {
        debug!("atefs::create parenet={}", parent);
        let mut inner = self.0.write().await;

        let entry = inner
            .inode_map
            .get(&parent)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::Dir(dir) = entry {
            let mut dir = dir.write().await;

            if dir.children.get(name).is_some() {
                return Err(libc::EEXIST.into());
            }

            let new_inode = inner.inode_gen.fetch_add(1, Ordering::Relaxed);

            let entry = Entry::File(Arc::new(RwLock::new(File {
                inode: new_inode,
                parent,
                name: name.to_os_string(),
                content: vec![],
                mode,
            })));

            let attr = entry.attr().await;

            dir.children.insert(name.to_os_string(), entry.clone());

            drop(dir);

            inner.inode_map.insert(new_inode, entry);

            Ok(ReplyCreated {
                ttl: TTL,
                attr,
                generation: 0,
                fh: 0,
                flags,
            })
        } else {
            Err(libc::ENOTDIR.into())
        }
    }

    async fn interrupt(&self, _req: Request, unique: u64) -> Result<()> {
        debug!("atefs::interrupt unique={}", unique);
        Ok(())
    }

    async fn fallocate(
        &self,
        _req: Request,
        inode: u64,
        _fh: u64,
        offset: u64,
        length: u64,
        _mode: u32,
    ) -> Result<()> {
        debug!("atefs::fallocate inode={}", inode);
        let inner = self.0.read().await;

        let entry = inner
            .inode_map
            .get(&inode)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::File(file) = entry {
            let mut file = file.write().await;

            let new_size = (offset + length) as usize;

            let size = file.content.len();

            if new_size > size {
                file.content.reserve(new_size - size);
            } else {
                file.content.truncate(new_size);
            }

            Ok(())
        } else {
            Err(libc::EISDIR.into())
        }
    }

    async fn readdirplus(
        &self,
        _req: Request,
        parent: u64,
        _fh: u64,
        offset: u64,
        _lock_owner: u64,
    ) -> Result<ReplyDirectoryPlus<Self::DirEntryPlusStream>> {        
        debug!("atefs::readdirplus id={} offset={}", parent, offset);
        let inner = self.0.read().await;

        let entry = inner
            .inode_map
            .get(&parent)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        if let Entry::Dir(dir) = entry {
            let attr = entry.attr().await;

            let dir = dir.read().await;

            let parent_attr = if dir.parent == dir.inode {
                attr
            } else {
                inner
                    .inode_map
                    .get(&dir.parent)
                    .expect("dir parent not exist")
                    .attr()
                    .await
            };

            let pre_children = stream::iter(
                vec![
                    (dir.inode, FileType::Directory, OsString::from("."), attr),
                    (
                        dir.parent,
                        FileType::Directory,
                        OsString::from(".."),
                        parent_attr,
                    ),
                ]
                .into_iter(),
            );

            let children = pre_children
                .chain(
                    stream::iter(dir.children.iter()).filter_map(|(name, entry)| async move {
                        let inode = entry.inode().await;
                        let attr = entry.attr().await;

                        Some((inode, entry.kind(), name.to_os_string(), attr))
                    }),
                )
                .map(|(inode, kind, name, attr)| DirectoryEntryPlus {
                    inode,
                    generation: 0,
                    kind,
                    name,
                    attr,
                    entry_ttl: TTL,
                    attr_ttl: TTL,
                })
                .skip(offset as _)
                .map(Ok)
                .collect::<Vec<_>>()
                .await;

            Ok(ReplyDirectoryPlus {
                entries: stream::iter(children),
            })
        } else {
            Err(libc::ENOTDIR.into())
        }
    }

    async fn readdir(
        &self,
        _req: Request,
        parent: u64,
        _fh: u64,
        offset: i64,
    ) -> Result<ReplyDirectory<Self::DirEntryStream>> {
        debug!("atefs::readdir parent={}", parent);
        let entries: Vec<Result<DirectoryEntry>> = vec![
            Ok(DirectoryEntry {
                inode: parent,
                kind: FileType::Directory,
                name: OsString::from("."),
            }),
            Ok(DirectoryEntry {
                inode: parent,
                kind: FileType::Directory,
                name: OsString::from(".."),
            }),
            Ok(DirectoryEntry {
                inode: parent,
                kind: FileType::RegularFile,
                name: OsString::from("blah".to_string()),
            }),
        ];

        Ok(ReplyDirectory {
            entries: stream::iter(entries.into_iter().skip(offset as usize)),
        })
    }

    async fn opendir(&self, _req: Request, inode: u64, _flags: u32) -> Result<ReplyOpen> {
        debug!("atefs::opendir inode={}", inode);
        Ok(ReplyOpen { fh: 0, flags: 0 })
    }

    async fn releasedir(&self, _req: Request, inode: u64, _fh: u64, _flags: u32) -> Result<()> {
        debug!("atefs::releasedir inode={}", inode);
        Ok(())
    }

    async fn rename2(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: &OsStr,
        _flags: u32,
    ) -> Result<()> {
        debug!("atefs::rename2");
        self.rename(req, parent, name, new_parent, new_name).await
    }

    async fn lseek(
        &self,
        _req: Request,
        inode: u64,
        _fh: u64,
        offset: u64,
        whence: u32,
    ) -> Result<ReplyLSeek> {
        debug!("atefs::lseek inode={}", inode);
        let inner = self.0.read().await;

        let entry = inner
            .inode_map
            .get(&inode)
            .ok_or_else(|| Errno::from(libc::ENOENT))?;

        let whence = whence as i32;

        if let Entry::File(file) = entry {
            let offset = if whence == libc::SEEK_CUR || whence == libc::SEEK_SET {
                offset
            } else if whence == libc::SEEK_END {
                let content_size = file.read().await.content.len();

                if content_size >= offset as _ {
                    content_size as u64 - offset
                } else {
                    0
                }
            } else {
                return Err(libc::EINVAL.into());
            };

            Ok(ReplyLSeek { offset })
        } else {
            Err(libc::EISDIR.into())
        }
    }

    async fn copy_file_range(
        &self,
        req: Request,
        inode: u64,
        fh_in: u64,
        off_in: u64,
        inode_out: u64,
        fh_out: u64,
        off_out: u64,
        length: u64,
        flags: u64,
    ) -> Result<ReplyCopyFileRange> {
        debug!("atefs::copy_file_range inode={}", inode);
        let data = self.read(req, inode, fh_in, off_in, length as _).await?;

        let data = data.data.as_ref().as_ref();

        let ReplyWrite { written } = self
            .write(req, inode_out, fh_out, off_out, data, flags as _)
            .await?;

        Ok(ReplyCopyFileRange { copied: written })
    }
}
*/