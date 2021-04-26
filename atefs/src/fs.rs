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

use ate::prelude::TransactionScope;
use ate::dio::Dio;
use ate::dio::Dao;
use ate::dio::DaoObjEthereal;
use ate::dio::DaoObjReal;
use ate::error::*;
use ate::chain::*;
use ate::session::Session as AteSession;
use ate::header::PrimaryKey;
use ate::prelude::AteHash;
use ate::prelude::AteSessionProperty;
use ate::prelude::AteRolePurpose;
use ate::meta::MetaAuthorization;
use crate::fixed::FixedFile;

use super::dir::Directory;
use super::file::RegularFile;
use super::model::*;
use super::api::*;
use crate::ConfAte;

use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use futures_util::stream;
use futures_util::stream::{Empty, Iter};
use futures_util::StreamExt;
use tokio::sync::RwLock;
use fxhash::FxHashMap;

use fuse3::raw::prelude::*;
use fuse3::{Errno, Result};

const FUSE_TTL: Duration = Duration::from_secs(1);

pub struct AteFS
where Self: Send + Sync
{
    pub chain: Arc<Chain>,
    pub no_auth: bool,
    pub scope: TransactionScope,
    pub group: Option<String>,
    pub session: AteSession,
    pub open_handles: Mutex<FxHashMap<u64, Arc<OpenHandle>>>,
    pub elapsed: std::time::Instant,
    pub last_elapsed: seqlock::SeqLock<u64>,
    pub commit_lock: tokio::sync::Mutex<()>,
}

pub struct OpenHandle
where Self: Send + Sync
{
    pub dirty: seqlock::SeqLock<bool>,

    pub inode: u64,
    pub fh: u64,
    pub attr: FileAttr,
    pub spec: FileSpec,
    
    pub children: Vec<DirectoryEntry>,
    pub children_plus: Vec<DirectoryEntryPlus>,
}

impl OpenHandle
{
    fn add_child(&mut self, spec: &FileSpec, uid: u32, gid: u32) {
        let attr = spec_as_attr(spec, uid, gid).clone();

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
            entry_ttl: FUSE_TTL,
            attr_ttl: FUSE_TTL,
        });
    }
}

pub fn spec_as_attr(spec: &FileSpec, uid: u32, gid: u32) -> FileAttr {
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
        uid,
        gid,
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

pub(crate) fn conv_commit<T>(r: std::result::Result<T, CommitError>) -> std::result::Result<T, Errno> {
    conv(match r {
        Ok(a) => Ok(a),
        Err(err) => Err(AteError::CommitError(err)),
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
            error!("atefs::error {}", err);
            match err {
                AteError::CommsError(CommsError::Disconnected) => Err(libc::EBUSY.into()),
                AteError::CommitError(CommitError::CommsError(CommsError::Disconnected)) => Err(libc::EBUSY.into()),
                AteError::LoadError(LoadError::NotFound(_)) => Err(libc::ENOENT.into()),
                AteError::LoadError(LoadError::TransformationError(TransformError::MissingReadKey(_))) => Err(libc::EACCES.into()),
                _ => Err(libc::EIO.into())
            }
        }
    }
}

impl AteFS
{
    pub fn new(chain: Arc<Chain>, group: Option<String>, session: AteSession, scope: TransactionScope, no_auth: bool) -> AteFS {
        AteFS {
            chain,
            no_auth,
            group,
            session,
            scope,
            open_handles: Mutex::new(FxHashMap::default()),
            elapsed: std::time::Instant::now(),
            last_elapsed: seqlock::SeqLock::new(0),
            commit_lock: tokio::sync::Mutex::new(()),
        }
    }

    fn get_group_gid(&self) -> Option<u32> {
        if let Some(group) = &self.group {
            self.session.get_group(group)
                .iter()
                .flat_map(|g| g.roles.iter())
                .filter_map(|r| r.gid())
                .map(|gid| gid.clone())
                .next()
        } else {
            None
        }
    }

    fn get_group_read_key(&self, gid: u32) -> Option<AteHash> {
        self.session.groups.iter()
            .flat_map(|g| g.roles.iter())
            .filter(|r| r.purpose == AteRolePurpose::Observer)
            .filter(|g| g.gid().iter().any(|gid2| *gid2 == gid))
            .flat_map(|r| r.read_keys())
            .map(|k| k.hash())
            .next()
    }

    fn get_group_write_key(&self, gid: u32) -> Option<AteHash> {
        self.session.groups.iter()
            .flat_map(|g| g.roles.iter())
            .filter(|r| r.purpose == AteRolePurpose::Contributor)
            .filter(|g| g.gid().iter().any(|gid2| *gid2 == gid))
            .flat_map(|r| r.write_keys())
            .map(|k| k.hash())
            .next()
    }

    fn get_user_read_key(&self, uid: u32) -> Option<AteHash> {
        match self.session.user.uid() {
            Some(a) if a == uid => {
                self.session.user
                    .read_keys()
                    .map(|a| a.hash())
                    .next()
            },
            _ => None
        }        
    }

    fn get_user_write_key(&self, uid: u32) -> Option<AteHash> {
        match self.session.user.uid() {
            Some(a) if a == uid => {
                self.session.user
                    .write_keys()
                    .map(|a| a.hash())
                    .next()
            },
            _ => None
        } 
    }

    pub async fn load<'a>(&'a self, inode: u64) -> Result<(Dao<Inode>, Dio<'a>)> {
        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        let dao = conv_load(dio.load::<Inode>(&PrimaryKey::from(inode)).await)?;
        Ok((dao, dio))
    }

    async fn create_open_handle(&self, inode: u64, req: &Request) -> Result<OpenHandle>
    {
        let key = PrimaryKey::from(inode);
        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        let data = conv_load(dio.load::<Inode>(&key).await)?;
        let created = data.when_created();
        let updated = data.when_updated();
        
        let uid = data.dentry.uid;
        let gid = data.dentry.gid;

        let mut children = Vec::new();
        let fixed = FixedFile::new(key.as_u64(), ".".to_string(), FileType::Directory)
            .uid(self.reverse_uid(uid, req))
            .gid(self.reverse_gid(gid, req))
            .created(created)
            .updated(updated);
        children.push(FileSpec::FixedFile(fixed));

        let fixed = FixedFile::new(key.as_u64(), "..".to_string(), FileType::Directory)
            .uid(self.reverse_uid(uid, req))
            .gid(self.reverse_gid(gid, req))
            .created(created)
            .updated(updated);
        children.push(FileSpec::FixedFile(fixed));

        for child in conv_load(data.iter(&mut dio, data.children).await)? {
            let child_spec = Inode::as_file_spec(child.key().as_u64(), child.when_created(), child.when_updated(), child);
            children.push(child_spec);
        }

        let spec = Inode::as_file_spec(key.as_u64(), created, updated, data);

        let mut open = OpenHandle {
            inode,
            fh: fastrand::u64(..),
            attr: spec_as_attr(&spec, self.reverse_uid(spec.uid(), req), self.reverse_gid(spec.uid(), req)),
            spec: spec,
            children: Vec::new(),
            children_plus: Vec::new(),
            dirty: seqlock::SeqLock::new(false),
        };

        for child in children.into_iter() {
            open.add_child(&child, self.reverse_uid(child.uid(), req), self.reverse_gid(child.gid(), req));
        }

        Ok(open)
    }
}

impl AteFS
{
    async fn mknod_internal<'a>(
        &'a self,
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _rdev: u32,
    ) -> Result<(Dao<Inode>, Dio<'a>)> {
        
        let key = PrimaryKey::from(parent);
        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        let mut data = conv_load(dio.load::<Inode>(&key).await)?;

        if data.spec_type != SpecType::Directory {
            debug!("atefs::create parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(_) = conv_load(data.iter(&mut dio, data.children).await)?.filter(|c| *c.dentry.name == *name).next() {
            debug!("atefs::create parent={} name={}: already-exists", parent, name.to_str().unwrap());
            return Err(libc::EEXIST.into());
        }

        let uid = self.translate_uid(req.uid, &req);
        let gid = self.translate_gid(req.gid, &req);
        let child = Inode::new(
            name.to_str().unwrap().to_string(),
            mode, 
            uid,
            gid,
            SpecType::RegularFile,
        );

        let mut child = conv_serialization(data.push_make(&mut dio, data.children, child))?;
        self.update_auth(mode, uid, gid, child.auth_mut())?;
        let child = conv_serialization(child.commit(&mut dio))?;
        return Ok((child, dio));
    }

    async fn tick(&self) -> Result<()> {
        let secs = self.elapsed.elapsed().as_secs();
        if secs > self.last_elapsed.read() {
            let _ = self.commit_lock.lock();
            if secs > self.last_elapsed.read() {
                *self.last_elapsed.lock_write() = secs;
                self.commit_internal().await?;
            }
        }
        Ok(())
    }

    async fn commit(&self) -> Result<()> {
        let _ = self.commit_lock.lock();
        self.commit_internal().await?;
        Ok(())
    }

    async fn commit_internal(&self) -> Result<()> {
        debug!("commit");
        let open_handles = {
            let lock = self.open_handles.lock();
            lock.values()
                .filter(|a| a.dirty.read())
                .map(|v| {
                    *v.dirty.lock_write() = false;
                    Arc::clone(v)
                })
                .collect::<Vec<_>>()
        };
        for open in open_handles {
            open.spec.commit(&self.chain, &self.session, self.scope).await?;
        }
        Ok(())
    }

    fn update_auth(&self, mode: u32, uid: u32, gid: u32, auth: &mut MetaAuthorization) -> Result<()> {
        if mode & 0o004 != 0 {
            auth.read = ate::meta::ReadOption::Everyone;
        } else if mode & 0o040 != 0 {
            if let Some(key) = self.get_group_read_key(gid) {
                auth.read = ate::meta::ReadOption::Specific(key);
            } else if self.no_auth == false {
                error!("Session does not have the required group ({}) read key embedded within it", gid);
                return Err(libc::EACCES.into());
            } else {
                auth.read = ate::meta::ReadOption::Inherit;
            }
        } else {
            if let Some(key) = self.get_user_read_key(uid) {
                auth.read = ate::meta::ReadOption::Specific(key);
            } else if self.no_auth == false {
                error!("Session does not have the required user ({}) read key embedded within it", uid);
                return Err(libc::EACCES.into());
            } else {
                auth.read = ate::meta::ReadOption::Inherit;
            }
        }

        if mode & 0o002 != 0 {
            auth.write = ate::meta::WriteOption::Everyone;
        } else if mode & 0o020 != 0 {
            if let Some(key) = self.get_group_write_key(gid) {
                auth.write = ate::meta::WriteOption::Specific(key);
            } else if self.no_auth == false {
                error!("Session does not have the required group ({}) rwrite key embedded within it", gid);
                return Err(libc::EACCES.into());
            } else {
                auth.write = ate::meta::WriteOption::Inherit;
            }
        } else {
            if let Some(key) = self.get_user_write_key(uid) {
                auth.write = ate::meta::WriteOption::Specific(key);
            } else if self.no_auth == false {
                error!("Session does not have the required user ({}) write key embedded within it", uid);
                return Err(libc::EACCES.into());
            } else {
                auth.write = ate::meta::WriteOption::Inherit;
            }
        }

        Ok(())
    }

    fn translate_uid(&self, uid: u32, req: &Request) -> u32 {
        if uid == 0 || uid == req.uid {
            return self.session.user.uid().unwrap_or_else(|| uid);
        }
        uid
    }

    fn translate_gid(&self, gid: u32, req: &Request) -> u32 {
        if gid == 0 || gid == req.gid {
            return self.get_group_gid().unwrap_or_else(|| gid);
        }
        gid
    }

    fn reverse_uid(&self, uid: u32, req: &Request) -> u32 {
        if let Some(uid2) = self.session.user.uid() {
            if uid == uid2 {
                return req.uid;
            }
        }
        uid
    }

    fn reverse_gid(&self, gid: u32, req: &Request) -> u32 {
        if let Some(gid2) = self.get_group_gid() {
            if gid == gid2 {
                return req.gid;
            }
        }
        gid
    }

    fn spec_as_attr_reverse(&self, spec: &FileSpec, req: &Request) -> FileAttr {
        let uid = self.reverse_uid(spec.uid(), req);
        let gid = self.reverse_gid(spec.gid(), req);
        spec_as_attr(spec, uid, gid)
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
        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        dio.auto_cancel();

        if let Err(LoadError::NotFound(_)) = dio.load::<Inode>(&PrimaryKey::from(1)).await {
            info!("atefs::creating-root-node");
            
            let mode = 0o755;
            let uid = self.translate_uid(req.uid, &req);
            let gid = self.translate_gid(req.gid, &req);
            let root = Inode::new(
                "/".to_string(),
                mode,
                uid,
                gid,
                SpecType::Directory
            );
            match dio.make_ext(root, self.session.log_format, Some(PrimaryKey::from(1))) {
                Ok(mut root) =>
                {
                    root.auto_cancel();
                    self.update_auth(mode, uid, gid, root.auth_mut())?;
                    conv_serialization(root.commit(&mut dio))?;
                },
                Err(err) => {
                    error!("atefs::error {}", err);
                }
            }     
        };
        debug!("atefs::init");
        
        // All good
        self.tick().await?;
        self.commit().await?;
        conv_commit(dio.commit().await)?;

        // Disable any more root nodes from being created (only the single root node is allowed)
        self.chain.single().await.disable_new_roots();

        Ok(())
    }

    async fn destroy(&self, _req: Request) {
        self.tick().await.unwrap();
        self.commit().await.unwrap();
        debug!("atefs::destroy");
    }

    async fn getattr(
        &self,
        req: Request,
        inode: u64,
        fh: Option<u64>,
        _flags: u32,
    ) -> Result<ReplyAttr> {
        self.tick().await?;
        debug!("atefs::getattr inode={}", inode);

        if let Some(fh) = fh {
            let lock = self.open_handles.lock();
            if let Some(open) = lock.get(&fh) {
                return Ok(ReplyAttr {
                    ttl: FUSE_TTL,
                    attr: open.attr,
                })
            }
        }

        let (dao, _dio) = self.load(inode).await?;
        let spec = Inode::as_file_spec(inode, dao.when_created(), dao.when_updated(), dao);
        Ok(ReplyAttr {
            ttl: FUSE_TTL,
            attr: self.spec_as_attr_reverse(&spec, &req),
        })
    }

    async fn setattr(
        &self,
        req: Request,
        inode: u64,
        _fh: Option<u64>,
        set_attr: SetAttr,
    ) -> Result<ReplyAttr> {
        self.tick().await?;
        debug!("atefs::setattr inode={}", inode);

        let key = PrimaryKey::from(inode);
        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        let mut dao = conv_load(dio.load::<Inode>(&key).await)?;

        dio.auto_cancel();
        dao.auto_cancel();

        let mut changed = false;
        if let Some(mode) = set_attr.mode {
            if dao.dentry.mode != mode {
                dao.dentry.mode = mode;
                changed = true;
            }
        }
        if let Some(uid) = set_attr.uid {
            let new_uid = self.translate_uid(uid, &req);
            if dao.dentry.uid != new_uid {
                dao.dentry.uid = new_uid;
                changed = true;
            }
        }
        if let Some(gid) = set_attr.gid {
            let new_gid = self.translate_gid(gid, &req);
            if dao.dentry.gid != new_gid {
                dao.dentry.gid = new_gid;
                changed = true;
            }
        }
        if changed == true {
            self.update_auth(dao.dentry.mode, dao.dentry.uid, dao.dentry.gid, dao.auth_mut())?;
        }

        conv_serialization(dao.commit(&mut dio))?;
        conv_commit(dio.commit().await)?;

        let spec = Inode::as_file_spec(inode, dao.when_created(), dao.when_updated(), dao);
        Ok(ReplyAttr {
            ttl: FUSE_TTL,
            attr: self.spec_as_attr_reverse(&spec, &req),
        })
    }

    async fn opendir(&self, req: Request, inode: u64, _flags: u32) -> Result<ReplyOpen> {
        self.tick().await?;
        debug!("atefs::opendir inode={}", inode);

        let open = self.create_open_handle(inode, &req).await?;

        if open.attr.kind != FileType::Directory {
            debug!("atefs::opendir not-a-directory");
            return Err(libc::ENOTDIR.into());
        }

        let fh = open.fh;
        self.open_handles.lock().insert(open.fh, Arc::new(open));

        Ok(ReplyOpen { fh, flags: 0 })
    }

    async fn releasedir(&self, _req: Request, inode: u64, fh: u64, _flags: u32) -> Result<()> {
        self.tick().await?;
        debug!("atefs::releasedir inode={}", inode);

        let open = self.open_handles.lock().remove(&fh);
        if let Some(open) = open {
            open.spec.commit(&self.chain, &self.session, self.scope).await?
        }
        Ok(())
    }

    async fn readdirplus(
        &self,
        req: Request,
        parent: u64,
        fh: u64,
        offset: u64,
        _lock_owner: u64,
    ) -> Result<ReplyDirectoryPlus<Self::DirEntryPlusStream>> {
        self.tick().await?;
        debug!("atefs::readdirplus id={} offset={}", parent, offset);

        if fh == 0 {
            let open = self.create_open_handle(parent, &req).await?;
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
        req: Request,
        parent: u64,
        fh: u64,
        offset: i64,
    ) -> Result<ReplyDirectory<Self::DirEntryStream>> {
        self.tick().await?;
        debug!("atefs::readdir parent={}", parent);

        if fh == 0 {
            let open = self.create_open_handle(parent, &req).await?;
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

    async fn lookup(&self, req: Request, parent: u64, name: &OsStr) -> Result<ReplyEntry> {
        self.tick().await?;
        let open = self.create_open_handle(parent, &req).await?;

        if open.attr.kind != FileType::Directory {
            debug!("atefs::lookup parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(entry) = open.children_plus.iter().filter(|c| *c.name == *name).next() {
            debug!("atefs::lookup parent={} name={}: found", parent, name.to_str().unwrap());
            return Ok(ReplyEntry {
                ttl: FUSE_TTL,
                attr: entry.attr,
                generation: 0,
            });
        }

        debug!("atefs::lookup parent={} name={}: not found", parent, name.to_str().unwrap());
        Err(libc::ENOENT.into())
    }

    async fn forget(&self, _req: Request, _inode: u64, _nlookup: u64) {
        let _ = self.tick().await;
    }

    async fn fsync(&self, _req: Request, inode: u64, _fh: u64, _datasync: bool) -> Result<()> {
        self.tick().await?;
        debug!("atefs::fsync inode={}", inode);

        Ok(())
    }

    async fn flush(&self, _req: Request, inode: u64, fh: u64, _lock_owner: u64) -> Result<()> {
        self.tick().await?;
        self.commit().await?;
        debug!("atefs::flush inode={}", inode);

        let open = {
            let lock = self.open_handles.lock();
            match lock.get(&fh) {
                Some(open) => Some(Arc::clone(&open)),
                _ => None,
            }
        };
        if let Some(open) = open {
            open.spec.commit(&self.chain, &self.session, self.scope).await?
        }

        conv_io(self.chain.flush().await)?;
        Ok(())
    }

    async fn access(&self, req: Request, inode: u64, mask: u32) -> Result<()> {
        self.tick().await?;
        debug!("atefs::access inode={}", inode);
        
        let (dao, _dio) = self.load(inode).await?;

        if (dao.dentry.mode & mask) != 0
        {
            return Ok(());
        }
        if req.uid == dao.dentry.uid {
            let mask_shift = mask << 3;
            if (dao.dentry.mode & mask_shift) != 0
            {
                return Ok(());
            }
        }
        if req.gid == dao.dentry.gid {
            let mask_shift = mask << 6;
            if (dao.dentry.mode & mask_shift) != 0
            {
                return Ok(());
            }
        }

        Err(libc::EACCES.into())
    }

    async fn mkdir(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
    ) -> Result<ReplyEntry> {
        self.tick().await?;
        debug!("atefs::mkdir parent={}", parent);

        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        dio.auto_cancel();

        let mut data = conv_load(dio.load::<Inode>(&PrimaryKey::from(parent)).await)?;
        
        if data.spec_type != SpecType::Directory {
            return Err(libc::ENOTDIR.into());
        }

        let uid = self.translate_uid(req.uid, &req);
        let gid = self.translate_gid(req.gid, &req);
        let child = Inode::new(
            name.to_str().unwrap().to_string(),
            mode, 
            uid,
            gid,
            SpecType::Directory,
        );

        let mut child = conv_serialization(data.push_make(&mut dio, data.children, child))?;
        child.auto_cancel();
        self.update_auth(mode, uid, gid, child.auth_mut())?;
        let child = conv_serialization(child.commit(&mut dio))?;
        
        let child_spec = Inode::as_file_spec(child.key().as_u64(), child.when_created(), child.when_updated(), child);
        conv_commit(dio.commit().await)?;

        Ok(ReplyEntry {
            ttl: FUSE_TTL,
            attr: self.spec_as_attr_reverse(&child_spec, &req),
            generation: 0,
        })
    }

    async fn rmdir(&self, req: Request, parent: u64, name: &OsStr) -> Result<()> {
        self.tick().await?;
        debug!("atefs::rmdir parent={}", parent);

        let open = self.create_open_handle(parent, &req).await?;

        if open.attr.kind != FileType::Directory {
            debug!("atefs::rmdir parent={} not-a-directory", parent);
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(entry) = open.children_plus.iter().filter(|c| *c.name == *name).next() {
            debug!("atefs::rmdir parent={} name={}: found", parent, name.to_str().unwrap());

            let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
            dio.auto_cancel();
            conv_load(dio.delete::<Inode>(&PrimaryKey::from(entry.inode)).await)?;
            conv_commit(dio.commit().await)?;
            return Ok(())
        }

        debug!("atefs::rmdir parent={} name={}: not found", parent, name.to_str().unwrap());
        Err(libc::ENOENT.into())
    }

    async fn interrupt(&self, _req: Request, unique: u64) -> Result<()> {
        self.tick().await?;
        debug!("atefs::interrupt unique={}", unique);

        Ok(())
    }

    async fn mknod(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        rdev: u32,
    ) -> Result<ReplyEntry> {
        self.tick().await?;
        debug!("atefs::mknod parent={} name={}", parent, name.to_str().unwrap().to_string());

        let (mut dao, mut dio) = self.mknod_internal(req, parent, name, mode, rdev).await?;
        dio.auto_cancel();
        dao.auto_cancel();

        conv_serialization(dao.commit(&mut dio))?;
        conv_commit(dio.commit().await)?;

        let spec = Inode::as_file_spec(dao.key().as_u64(), dao.when_created(), dao.when_updated(), dao);
        Ok(ReplyEntry {
            ttl: FUSE_TTL,
            attr: self.spec_as_attr_reverse(&spec, &req),
            generation: 0,
        })
    }

    async fn create(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> Result<ReplyCreated> {
        self.tick().await?;
        debug!("atefs::create parent={} name={}", parent, name.to_str().unwrap().to_string());

        let (mut data, mut dio) = self.mknod_internal(req, parent, name, mode, 0).await?;
        dio.auto_cancel();
        data.auto_cancel();

        conv_serialization(data.commit(&mut dio))?;
        let spec = Inode::as_file_spec(data.key().as_u64(), data.when_created(), data.when_updated(), data);

        let open = OpenHandle {
            inode: spec.ino(),
            fh: fastrand::u64(..),
            attr: self.spec_as_attr_reverse(&spec, &req),
            spec: spec,
            children: Vec::new(),
            children_plus: Vec::new(),
            dirty: seqlock::SeqLock::new(false),
        };

        let fh = open.fh;
        let attr = open.attr.clone();

        conv_commit(dio.commit().await)?;
        self.open_handles.lock().insert(open.fh, Arc::new(open));

        Ok(ReplyCreated {
            ttl: FUSE_TTL,
            attr: attr,
            generation: 0,
            fh,
            flags,
        })
    }

    async fn unlink(&self, _req: Request, parent: u64, name: &OsStr) -> Result<()> {
        self.tick().await?;
        debug!("atefs::unlink parent={} name={}", parent, name.to_str().unwrap().to_string());

        let parent_key = PrimaryKey::from(parent);
        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        dio.auto_cancel();

        let data_parent = conv_load(dio.load::<Inode>(&parent_key).await)?;

        if data_parent.spec_type != SpecType::Directory {
            debug!("atefs::unlink parent={} not-a-directory", parent);
            
            dio.cancel();
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(data) = conv_load(data_parent.iter(&mut dio, data_parent.children).await)?.filter(|c| *c.dentry.name == *name).next()
        {
            if data.spec_type == SpecType::Directory {
                debug!("atefs::unlink parent={} name={} is-a-directory", parent, name.to_str().unwrap().to_string());
                
                dio.cancel();
                return Err(libc::EISDIR.into());
            }

            conv_serialization(data.delete(&mut dio))?;
            conv_commit(dio.commit().await)?;
            return Ok(());
        }

        dio.cancel();
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
        self.tick().await?;
        debug!("atefs::rename name={} new_name={}", name.to_str().unwrap().to_string(), new_name.to_str().unwrap().to_string());
        
        let mut dio = self.chain.dio_ext(&self.session, self.scope).await;
        let parent_key = PrimaryKey::from(parent);
        let parent_data = conv_load(dio.load::<Inode>(&parent_key).await)?;

        if parent_data.spec_type != SpecType::Directory {
            debug!("atefs::rename parent={} not-a-directory", parent);
            dio.cancel();
            return Err(libc::ENOTDIR.into());
        }
        
        if let Some(mut data) = conv_load(parent_data.iter(&mut dio, parent_data.children).await)?.filter(|c| *c.dentry.name == *name).next()
        {
            // If the parent has changed then move it
            if parent != new_parent
            {
                let new_parent_key = PrimaryKey::from(new_parent);
                let new_parent_data = conv_load(dio.load::<Inode>(&new_parent_key).await)?;

                if new_parent_data.spec_type != SpecType::Directory {
                    debug!("atefs::rename new_parent={} not-a-directory", new_parent);
                    dio.cancel();
                    return Err(libc::ENOTDIR.into());
                }

                if conv_load(new_parent_data.iter(&mut dio, new_parent_data.children).await)?.filter(|c| *c.dentry.name == *new_name).next().is_some() {
                    debug!("atefs::rename new_name={} already exists", new_name.to_str().unwrap().to_string());
                    dio.cancel();
                    return Err(libc::EEXIST.into());
                }

                data.detach();
                data.attach(&new_parent_data, new_parent_data.children);
            }
            else
            {
                if conv_load(parent_data.iter(&mut dio, parent_data.children).await)?.filter(|c| *c.dentry.name == *new_name).next().is_some() {
                    debug!("atefs::rename new_name={} already exists", new_name.to_str().unwrap().to_string());
                    dio.cancel();
                    return Err(libc::ENOTDIR.into());
                }
            }

            data.dentry.name = new_name.to_str().unwrap().to_string();
            conv_serialization(data.commit(&mut dio))?;
            conv_commit(dio.commit().await)?;
            return Ok(());
        }

        dio.cancel();
        Err(libc::ENOENT.into())
    }

    async fn open(&self, req: Request, inode: u64, flags: u32) -> Result<ReplyOpen> {
        self.tick().await?;
        debug!("atefs::open inode={}", inode);

        let open = self.create_open_handle(inode, &req).await?;

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
        self.tick().await?;
        debug!("atefs::release inode={}", inode);
        
        
        let open = self.open_handles.lock().remove(&fh);
        if let Some(open) = open {
            open.spec.commit(&self.chain, &self.session, self.scope).await?
        }

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
        self.tick().await?;
        debug!("atefs::read inode={} offset={} size={}", inode, offset, size);
        
        let open = {
            let lock = self.open_handles.lock();
            match lock.get(&fh) {
                Some(a) => Arc::clone(a),
                None => {
                    return Err(libc::ENOSYS.into());
                },
            }
        };
        Ok(ReplyData { data: open.spec.read(&self.chain, &self.session, self.scope, offset, size as u64).await?,  })
    }

    async fn write(
        &self,
        _req: Request,
        inode: u64,
        fh: u64,
        offset: u64,
        data: &[u8],
        _flags: u32,
    ) -> Result<ReplyWrite> {
        self.tick().await?;
        debug!("atefs::write inode={} offset={} size={}", inode, offset, data.len());

        let open = {
            let lock = self.open_handles.lock();
            match lock.get(&fh) {
                Some(a) => Arc::clone(a),
                None => {
                    debug!("atefs::write-failed inode={} offset={} size={}", inode, offset, data.len());
                    return Err(libc::ENOSYS.into());
                },
            }
        };

        let wrote = open.spec.write(&self.chain, &self.session, self.scope, offset, data).await?;
        if open.dirty.read() == false {
            *open.dirty.lock_write() = true;
        }

        debug!("atefs::wrote inode={} offset={} size={}", inode, offset, wrote);
        Ok(ReplyWrite {
            written: wrote,
        })
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
        self.tick().await?;
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
                open.spec.fallocate(&self.chain, &self.session, self.scope, offset + length).await?;
                if open.dirty.read() == false {
                    *open.dirty.lock_write() = true;
                }
                return Ok(());
            }
        }

        let (mut dao, mut dio) = self.load(inode).await?;
        dao.size = offset + length;
        conv_serialization(dao.commit(&mut dio))?;
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
        self.tick().await?;
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
                None => self.load(inode).await?.0.size
            };
            offset + size
        } else {
            return Err(libc::EINVAL.into());
        };
        Ok(ReplyLSeek { offset })
    }

    async fn symlink(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        link: &OsStr,
    ) -> Result<ReplyEntry> {
        self.tick().await?;
        debug!("atefs::symlink parent={}, name={}, link={}", parent, name.to_str().unwrap().to_string(), link.to_str().unwrap().to_string());

        let link = link.to_str().unwrap().to_string();
        let spec = {
            let (mut dao, mut dio) = self.mknod_internal(req, parent, name, 0o755, 0).await?;
            dio.auto_cancel();
            dao.auto_cancel();

            dao.spec_type = SpecType::SymLink;
            dao.link = Some(link);
            conv_serialization(dao.commit(&mut dio))?;
            conv_commit(dio.commit().await)?;
            Inode::as_file_spec(dao.key().as_u64(), dao.when_created(), dao.when_updated(), dao)
        };
        
        Ok(ReplyEntry {
            ttl: FUSE_TTL,
            attr: self.spec_as_attr_reverse(&spec, &req),
            generation: 0,
        })
    }

    /// read symbolic link.
    async fn readlink(
        &self,
        _req: Request,
        inode: u64
    ) -> Result<ReplyData> {
        self.tick().await?;
        debug!("atefs::readlink inode={}", inode);

        let dao = self.load(inode).await?;
        match &dao.0.link {
            Some(l) => {
                Ok(ReplyData {
                    data: bytes::Bytes::from(l.clone().into_bytes()),
                })
            },
            None => Err(libc::ENOSYS.into())
        }
    }

    /// create a hard link.
    async fn link(
        &self,
        _req: Request,
        _inode: u64,
        _new_parent: u64,
        _new_name: &OsStr,
    ) -> Result<ReplyEntry> {
        self.tick().await?;
        debug!("atefs::link not-implemented");

        Err(libc::ENOSYS.into())
    }

    /// get filesystem statistics.
    async fn statsfs(
        &self,
        _req: Request,
        _inode: u64
    ) -> Result<ReplyStatFs> {
        self.tick().await?;
        debug!("atefs::statsfs not-implemented");

        Err(libc::ENOSYS.into())
    }

    /// set an extended attribute.
    async fn setxattr(
        &self,
        _req: Request,
        _inode: u64,
        _name: &OsStr,
        _value: &OsStr,
        _flags: u32,
        _position: u32,
    ) -> Result<()> {
        self.tick().await?;
        debug!("atefs::setxattr not-implemented");
        
        Err(libc::ENOSYS.into())
    }

    /// get an extended attribute. If size is too small, use [`ReplyXAttr::Size`] to return correct
    /// size. If size is enough, use [`ReplyXAttr::Data`] to send it, or return error.
    async fn getxattr(
        &self,
        _req: Request,
        _inode: u64,
        _name: &OsStr,
        _size: u32,
    ) -> Result<ReplyXAttr> {
        self.tick().await?;
        debug!("atefs::getxattr not-implemented");

        Err(libc::ENOSYS.into())
    }

    /// list extended attribute names. If size is too small, use [`ReplyXAttr::Size`] to return
    /// correct size. If size is enough, use [`ReplyXAttr::Data`] to send it, or return error.
    async fn listxattr(
        &self,
        _req: Request,
        _inode: u64,
        _size: u32
    ) -> Result<ReplyXAttr> {
        self.tick().await?;
        debug!("atefs::listxattr not-implemented");

        Err(libc::ENOSYS.into())
    }

    /// remove an extended attribute.
    async fn removexattr(
        &self,
        _req: Request,
        _inode: u64,
        _name: &OsStr
    ) -> Result<()> {
        self.tick().await?;
        debug!("atefs::removexattr not-implemented");

        Err(libc::ENOSYS.into())
    }

    /// map block index within file to block index within device.
    ///
    /// # Notes:
    ///
    /// This may not works because currently this crate doesn't support fuseblk mode yet.
    async fn bmap(
        &self,
        _req: Request,
        _inode: u64,
        _blocksize: u32,
        _idx: u64,
    ) -> Result<ReplyBmap> {
        self.tick().await?;
        debug!("atefs::bmap not-implemented");

        Err(libc::ENOSYS.into())
    }

    async fn poll(
        &self,
        _req: Request,
        _inode: u64,
        _fh: u64,
        _kh: Option<u64>,
        _flags: u32,
        _events: u32,
        _notify: &Notify,
    ) -> Result<ReplyPoll> {
        self.tick().await?;
        debug!("atefs::poll not-implemented");

        Err(libc::ENOSYS.into())
    }

    async fn notify_reply(
        &self,
        _req: Request,
        _inode: u64,
        _offset: u64,
        _data: bytes::Bytes,
    ) -> Result<()> {
        self.tick().await?;
        debug!("atefs::notify_reply not-implemented");

        Err(libc::ENOSYS.into())
    }

    /// forget more than one inode. This is a batch version [`forget`][Filesystem::forget]
    async fn batch_forget(
        &self,
        _req: Request,
        _inodes: &[u64])
    {
        let _ = self.tick().await;
        debug!("atefs::batch_forget not-implemented");
    }

    async fn copy_file_range(
        &self,
        _req: Request,
        _inode: u64,
        _fh_in: u64,
        _off_in: u64,
        _inode_out: u64,
        _fh_out: u64,
        _off_out: u64,
        _length: u64,
        _flags: u64,
    ) -> Result<ReplyCopyFileRange> {
        self.tick().await?;
        debug!("atefs::copy_file_range not-implemented");

        Err(libc::ENOSYS.into())
    }
}