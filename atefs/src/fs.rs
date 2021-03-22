#![allow(unused_imports)]
use log::{info, error, debug};

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
use crate::fixed::FixedFile;

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
    pub open_handles: Mutex<FxHashMap<u64, Arc<OpenHandle>>>
}

pub struct OpenHandle
where Self: Send + Sync
{
    pub inode: u64,
    pub fh: u64,
    pub attr: FileAttr,
    pub spec: FileSpec,
    pub children: Vec<DirectoryEntry>,
    pub children_plus: Vec<DirectoryEntryPlus>,
}

impl OpenHandle
{
    fn add_child(&mut self, spec: &FileSpec) {
        let attr = spec_as_attr(spec).clone();

        self.children.push(DirectoryEntry {
            inode: spec.ino(),
            kind: spec.kind(),
            name: OsString::from(spec.name()),
        });
        self.children_plus.push(DirectoryEntryPlus {
            inode: spec.ino(),
            kind: spec.kind(),
            name: OsString::from(spec.name().clone()),
            generation: 0,
            attr,
            entry_ttl: TTL,
            attr_ttl: TTL,
        });
    }
}

pub fn spec_as_attr(spec: &FileSpec) -> FileAttr {
    let size = spec.size();
    let blksize = super::model::PAGE_SIZE as u64;

    FileAttr {
        ino: spec.ino(),
        generation: 0,
        size,
        blocks: (size / blksize),
        atime: SystemTime::UNIX_EPOCH + Duration::from_millis(spec.accessed()),
        mtime: SystemTime::UNIX_EPOCH + Duration::from_millis(spec.updated()),
        ctime: SystemTime::UNIX_EPOCH + Duration::from_millis(spec.created()),
        kind: spec.kind(),
        perm: fuse3::perm_from_mode_and_kind(spec.kind(), spec.mode()),
        nlink: 0,
        uid: spec.uid(),
        gid: spec.gid(),
        rdev: 0,
        blksize: blksize as u32,
    }
}

pub(crate) fn conv_load<T>(r: std::result::Result<T, LoadError>) -> std::result::Result<T, Errno> {
    conv(match r {
        Ok(a) => Ok(a),
        Err(err) => Err(AteError::LoadError(err)),
    })
}

pub(crate) fn conv_io<T>(r: std::result::Result<T, tokio::io::Error>) -> std::result::Result<T, Errno> {
    conv(match r {
        Ok(a) => Ok(a),
        Err(err) => Err(AteError::IO(err)),
    })
}

pub(crate) fn conv_serialization<T>(r: std::result::Result<T, SerializationError>) -> std::result::Result<T, Errno> {
    conv(match r {
        Ok(a) => Ok(a),
        Err(err) => Err(AteError::SerializationError(err)),
    })
}

pub(crate) fn conv<T>(r: std::result::Result<T, AteError>) -> std::result::Result<T, Errno> {
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

    pub async fn load(&self, inode: u64) -> Result<Dao<Inode>> {
        let mut dio = self.chain.dio(&self.session).await;
        let dao = conv_load(dio.load::<Inode>(&PrimaryKey::from(inode)).await)?;
        Ok(dao)
    }

    async fn create_open_handle(&self, inode: u64) -> Result<OpenHandle>
    {
        let key = PrimaryKey::from(inode);
        let mut dio = self.chain.dio(&self.session).await;
        let data = conv_load(dio.load::<Inode>(&key).await)?;
        let created = data.when_created();
        let updated = data.when_updated();
        
        let uid = data.dentry.uid;
        let gid = data.dentry.gid;

        let mut children = Vec::new();
        let fixed = FixedFile::new(key.as_u64(), ".".to_string(), FileType::Directory)
            .uid(uid)
            .gid(gid)
            .created(created)
            .updated(updated);
        children.push(FileSpec::FixedFile(fixed));

        let fixed = FixedFile::new(key.as_u64(), "..".to_string(), FileType::Directory)
            .uid(uid)
            .gid(gid)
            .created(created)
            .updated(updated);
        children.push(FileSpec::FixedFile(fixed));

        for child in conv_load(data.children.iter(&key, &mut dio).await)? {
            let child_spec = Inode::as_file_spec(child.key().as_u64(), child.when_created(), child.when_updated(), child);
            children.push(child_spec);
        }

        let spec = Inode::as_file_spec(key.as_u64(), created, updated, data);

        let mut open = OpenHandle {
            inode,
            fh: fastrand::u64(..),
            attr: spec_as_attr(&spec),
            spec: spec,
            children: Vec::new(),
            children_plus: Vec::new(),
        };

        for child in children.into_iter() {
            open.add_child(&child);
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

    async fn init(&self, req: Request) -> Result<()>
    {
        // Attempt to load the root node, if it does not exist then create it
        //let mut dio = self.chain.dio_ext(&self.session, Scope::Full).await;
        let mut dio = self.chain.dio(&self.session).await;
        if let Err(LoadError::NotFound(_)) = dio.load::<Inode>(&PrimaryKey::from(1)).await {
            info!("atefs::creating-root-node");
            
            let root = Inode::new("/".to_string(), 0o755, req.uid, req.gid, SpecType::Directory);
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
        let spec = Inode::as_file_spec(inode, dao.when_created(), dao.when_updated(), dao);
        Ok(ReplyAttr {
            ttl: TTL,
            attr: spec_as_attr(&spec),
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

        let key = PrimaryKey::from(inode);
        let mut dio = self.chain.dio(&self.session).await;
        let mut dao = conv_load(dio.load::<Inode>(&key).await)?;

        if let Some(mode) = set_attr.mode {
            dao.dentry.mode = mode;
        }
        if let Some(uid) = set_attr.uid {
            dao.dentry.uid = uid;
        }
        if let Some(gid) = set_attr.gid {
            dao.dentry.gid = gid;
        }

        let spec = Inode::as_file_spec(inode, dao.when_created(), dao.when_updated(), dao);
        Ok(ReplyAttr {
            ttl: TTL,
            attr: spec_as_attr(&spec),
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
        self.open_handles.lock().insert(open.fh, Arc::new(open));

        Ok(ReplyOpen { fh, flags: 0 })
    }

    async fn releasedir(&self, _req: Request, inode: u64, fh: u64, _flags: u32) -> Result<()> {
        debug!("atefs::releasedir inode={}", inode);
        self.open_handles.lock().remove(&fh);
        Ok(())
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
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
    ) -> Result<ReplyEntry> {
        debug!("atefs::mkdir parent={}", parent);

        let key = PrimaryKey::from(parent);
        let mut dio = self.chain.dio(&self.session).await;
        let data = conv_load(dio.load::<Inode>(&PrimaryKey::from(parent)).await)?;
        
        if data.spec_type != SpecType::Directory {
            return Err(libc::ENOTDIR.into());
        }

        let child = Inode::new(
            name.to_str().unwrap().to_string(),
            mode, 
            req.uid,
            req.gid,
            SpecType::Directory,
        );

        let child = conv_serialization(data.children.push(&mut dio, &key, child))?;
        let child_spec = Inode::as_file_spec(child.key().as_u64(), child.when_created(), child.when_updated(), child);

        Ok(ReplyEntry {
            ttl: TTL,
            attr: spec_as_attr(&child_spec),
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

            conv_serialization(data.delete())?;

            return Ok(())
        }

        debug!("atefs::rmdir parent={} name={}: not found", parent, name.to_str().unwrap());
        Err(libc::ENOENT.into())
    }

    async fn interrupt(&self, _req: Request, unique: u64) -> Result<()> {
        debug!("atefs::interrupt unique={}", unique);
        Ok(())
    }

    async fn mknod(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _rdev: u32,
    ) -> Result<ReplyEntry> {
        debug!("atefs::mknod parent={} name={}", parent, name.to_str().unwrap().to_string());

        let key = PrimaryKey::from(parent);
        let mut dio = self.chain.dio(&self.session).await;
        let data = conv_load(dio.load::<Inode>(&key).await)?;

        if data.spec_type != SpecType::Directory {
            debug!("atefs::create parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(_) = conv_load(data.children.iter(&key, &mut dio).await)?.filter(|c| *c.dentry.name == *name).next() {
            debug!("atefs::create parent={} name={}: already-exists", parent, name.to_str().unwrap());
            return Err(libc::EEXIST.into());
        }

        let child = Inode::new(
            name.to_str().unwrap().to_string(),
            mode, 
            req.uid,
            req.gid,
            SpecType::RegularFile,
        );
        conv_serialization(data.children.push(&mut dio, &key, child))?;

        let spec = Inode::as_file_spec(data.key().as_u64(), data.when_created(), data.when_updated(), data);
        let attr = spec_as_attr(&spec);

        Ok(ReplyEntry {
            ttl: TTL,
            attr,
            generation: 0,
        })
    }

    async fn unlink(&self, _req: Request, parent: u64, name: &OsStr) -> Result<()> {
        debug!("atefs::unlink parent={} name={}", parent, name.to_str().unwrap().to_string());

        let key = PrimaryKey::from(parent);
        let mut dio = self.chain.dio(&self.session).await;
        let data = conv_load(dio.load::<Inode>(&key).await)?;

        if data.spec_type != SpecType::Directory {
            debug!("atefs::unlink parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(data) = conv_load(data.children.iter(&key, &mut dio).await)?.filter(|c| *c.dentry.name == *name).next()
        {
            if data.spec_type == SpecType::Directory {
                debug!("atefs::unlink parent={} name={} is-a-directory", parent, name.to_str().unwrap().to_string());
                return Err(libc::EISDIR.into());
            }

            conv_serialization(data.delete())?;

            return Ok(());
        }

        Err(libc::ENOENT.into())
    }

    async fn rename(
        &self,
        _req: Request,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: &OsStr,
    ) -> Result<()> {
        debug!("atefs::rename name={} new_name={}", name.to_str().unwrap().to_string(), new_name.to_str().unwrap().to_string());
        
        let mut dio = self.chain.dio(&self.session).await;
        let parent_key = PrimaryKey::from(parent);
        let parent_data = conv_load(dio.load::<Inode>(&parent_key).await)?;

        if parent_data.spec_type != SpecType::Directory {
            debug!("atefs::rename parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(mut data) = conv_load(parent_data.children.iter(&parent_key, &mut dio).await)?.filter(|c| *c.dentry.name == *name).next()
        {
            // If the parent has changed then move it
            if parent != new_parent
            {
                let new_parent_key = PrimaryKey::from(new_parent);
                let new_parent_data = conv_load(dio.load::<Inode>(&new_parent_key).await)?;

                if new_parent_data.spec_type != SpecType::Directory {
                    debug!("atefs::rename new_parent={} not-a-directory", new_parent);
                    return Err(libc::ENOTDIR.into());
                }

                if conv_load(new_parent_data.children.iter(&parent_key, &mut dio).await)?.filter(|c| *c.dentry.name == *new_name).next().is_some() {
                    debug!("atefs::rename new_name={} already exists", new_name.to_str().unwrap().to_string());
                    return Err(libc::EEXIST.into());
                }

                data.detach();
                data.attach(&new_parent_key, &new_parent_data.children);
            }
            else
            {
                if conv_load(parent_data.children.iter(&parent_key, &mut dio).await)?.filter(|c| *c.dentry.name == *new_name).next().is_some() {
                    debug!("atefs::rename new_name={} already exists", new_name.to_str().unwrap().to_string());
                    return Err(libc::ENOTDIR.into());
                }
            }

            data.dentry.name = new_name.to_str().unwrap().to_string();

            return Ok(());
        }

        Err(libc::ENOENT.into())
    }

    async fn open(&self, _req: Request, inode: u64, flags: u32) -> Result<ReplyOpen> {
        debug!("atefs::open inode={}", inode);

        let open = self.create_open_handle(inode).await?;

        if open.attr.kind == FileType::Directory {
            debug!("atefs::open is-a-directory");
            return Err(libc::EISDIR.into());
        }

        let fh = open.fh;
        self.open_handles.lock().insert(open.fh, Arc::new(open));

        Ok(ReplyOpen { fh, flags })
    }

    async fn release(
        &self,
        _req: Request,
        inode: u64,
        fh: u64,
        _flags: u32,
        _lock_owner: u64,
        flush: bool,
    ) -> Result<()> {
        debug!("atefs::release inode={}", inode);
        self.open_handles.lock().remove(&fh);

        if flush {
            self.chain.flush().await?;
        }

        Ok(())
    }

    async fn read(
        &self,
        _req: Request,
        inode: u64,
        fh: u64,
        offset: u64,
        size: u32,
    ) -> Result<ReplyData> {
        debug!("atefs::read inode={}", inode);
        
        let open = {
            let lock = self.open_handles.lock();
            match lock.get(&fh) {
                Some(a) => Arc::clone(a),
                None => {
                    return Err(libc::ENOSYS.into());
                },
            }
        };
        Ok(ReplyData { data: open.spec.read(&self.chain, &self.session, offset, size).await?,  })
    }

    async fn fallocate(
        &self,
        _req: Request,
        inode: u64,
        fh: u64,
        offset: u64,
        length: u64,
        _mode: u32,
    ) -> Result<()> {
        debug!("atefs::fallocate inode={}", inode);

        if fh > 0 {
            let open = {
                let lock = self.open_handles.lock();
                match lock.get(&fh) {
                    Some(a) => Some(Arc::clone(a)),
                    None => None,
                }
            };
            if let Some(open) = open {
                open.spec.fallocate(offset + length).await;
                return Ok(());
            }
        }

        let mut dao = self.load(inode).await?;
        dao.size = offset + length;
        return Ok(());
    }

    async fn lseek(
        &self,
        _req: Request,
        inode: u64,
        fh: u64,
        offset: u64,
        whence: u32,
    ) -> Result<ReplyLSeek> {
        debug!("atefs::lseek inode={}", inode);

        let offset = if whence == libc::SEEK_CUR as u32 || whence == libc::SEEK_SET as u32 {
            offset
        } else if whence == libc::SEEK_END as u32 {
            let mut size = None;
            if fh > 0 {
                let lock = self.open_handles.lock();
                if let Some(open) = lock.get(&fh) {
                    size = Some(open.spec.size());
                }
            }
            let size = match size {
                Some(a) => a,
                None => self.load(inode).await?.size
            };
            offset + size
        } else {
            return Err(libc::EINVAL.into());
        };
        Ok(ReplyLSeek { offset })
    }
}

/*
#[async_trait]
impl Filesystem for AteFS {
    type DirEntryStream = Iter<std::iter::Skip<IntoIter<Result<DirectoryEntry>>>>;
    type DirEntryPlusStream = Iter<IntoIter<Result<DirectoryEntryPlus>>>;


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
}
*/