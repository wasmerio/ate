#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;

use std::sync::Arc;
use parking_lot::Mutex;
use bytes::Bytes;

use ::ate::{crypto::DerivedEncryptKey, prelude::TransactionScope};
use ::ate::dio::Dio;
use ::ate::dio::Dao;
use ::ate::dio::DaoObj;
use ::ate::chain::*;
use ::ate::crypto::*;
use ::ate::header::PrimaryKey;
use ::ate::prelude::*;
use ::ate::prelude::AteRolePurpose;
use ::ate::prelude::ReadOption;
use crate::fixed::FixedFile;

use super::model::*;
use super::api::*;
use super::handle::*;
use super::error::*;
use super::prelude::*;

use fxhash::FxHashMap;

pub struct FileAccessor
where Self: Send + Sync
{
    pub chain: Arc<Chain>,
    pub dio: Arc<Dio>,
    pub no_auth: bool,
    pub is_www: bool,
    pub is_edge: bool,
    pub scope_meta: TransactionScope,
    pub scope_io: TransactionScope,
    pub group: Option<String>,
    pub session: AteSessionType,
    pub open_handles: Mutex<FxHashMap<u64, Arc<OpenHandle>>>,
    pub elapsed: std::time::Instant,
    pub last_elapsed: seqlock::SeqLock<u64>,
    pub commit_lock: tokio::sync::Mutex<()>,
    pub impersonate_uid: bool,
}

pub struct RequestContext
{
    pub uid: u32,
    pub gid: u32,
}

impl Default
for RequestContext
{
    fn default() -> RequestContext {
        RequestContext {
            uid: 0u32,
            gid: 0u32,
        }
    }
}

impl FileAccessor
{
    pub async fn new(chain: Arc<Chain>, group: Option<String>, session: AteSessionType, scope_io: TransactionScope, scope_meta: TransactionScope, no_auth: bool, impersonate_uid: bool) -> FileAccessor {
        let is_www = chain.key().to_string().ends_with("/www");
        let is_edge = chain.key().to_string().ends_with("/edge");
        let dio = chain.dio(&session).await;

        FileAccessor {
            chain,
            dio,
            no_auth,
            is_www,
            is_edge,
            group,
            session,
            scope_meta,
            scope_io,
            open_handles: Mutex::new(FxHashMap::default()),
            elapsed: std::time::Instant::now(),
            last_elapsed: seqlock::SeqLock::new(0),
            commit_lock: tokio::sync::Mutex::new(()),
            impersonate_uid,
        }
    }

    pub async fn init(&self, req: &RequestContext) -> Result<()>
    {
        let dio = self.dio_mut_meta().await;
        if let Err(LoadError(LoadErrorKind::NotFound(_), _)) = self.dio.load::<Inode>(&PrimaryKey::from(1)).await {
            info!("creating-root-node");
            
            let mode = 0o770;
            let uid = self.translate_uid(req.uid, req);
            let gid = self.translate_gid(req.gid, req);
            let root = Inode::new(
                "/".to_string(),
                mode,
                uid,
                gid,
                FileKind::Directory
            );
            match dio.store_with_key(root, PrimaryKey::from(1))
            {
                Ok(mut root) => {
                    self.update_auth(mode, uid, gid, root.auth_mut())?;
                },
                Err(err) => {
                    error!("{}", err);
                }
            }     
        };
        debug!("init");
        
        // All good
        self.tick().await?;
        self.commit().await?;
        dio.commit().await?;

        // Disable any more root nodes from being created (only the single root node is allowed)
        self.chain.single().await.disable_new_roots();
        Ok(())
    }

    pub fn get_group_read_key<'a>(&'a self, gid: u32) -> Option<&'a EncryptKey> {
        let purpose = if self.is_www {
            AteRolePurpose::WebServer
        } else if self.is_edge {
            AteRolePurpose::EdgeCompute
        } else {
            AteRolePurpose::Observer
        };
        self.session.role(&purpose)
            .iter()
            .filter(|g| g.gid() == Some(gid))
            .flat_map(|r| r.read_keys())
            .next()
    }

    pub fn get_group_write_key<'a>(&'a self, gid: u32) -> Option<&'a PrivateSignKey> {
        let purpose = if self.is_www {
            AteRolePurpose::WebServer
        } else if self.is_edge {
            AteRolePurpose::EdgeCompute
        } else {
            AteRolePurpose::Contributor
        };
        self.session.role(&purpose)
            .iter()
            .filter(|g| g.gid() == Some(gid))
            .flat_map(|r| r.write_keys())
            .next()
    }

    pub fn get_user_read_key<'a>(&'a self, uid: u32) -> Option<&'a EncryptKey> {
        if self.session.uid() == Some(uid) {
            match &self.session {
                AteSessionType::User(a) => a.user.read_keys().next(),
                AteSessionType::Sudo(a) => a.inner.user.read_keys().next(),
                AteSessionType::Group(a) => match &a.inner {
                    AteSessionInner::User(a) => a.user.read_keys().next(),
                    AteSessionInner::Sudo(a) => a.inner.user.read_keys().next(),
                },
            }
        } else {
            None
        }
    }

    pub fn get_user_write_key<'a>(&'a self, uid: u32) -> Option<&'a PrivateSignKey> {
        if self.session.uid() == Some(uid) {
            match &self.session {
                AteSessionType::User(a) => a.user.write_keys().next(),
                AteSessionType::Sudo(a) => a.inner.user.write_keys().next(),
                AteSessionType::Group(a) => match &a.inner {
                    AteSessionInner::User(a) => a.user.write_keys().next(),
                    AteSessionInner::Sudo(a) => a.inner.user.write_keys().next(),
                },
            }
        } else {
            None
        }
    }

    pub async fn load(&self, inode: u64) -> Result<Dao<Inode>> {
        let dao = self.dio.load::<Inode>(&PrimaryKey::from(inode)).await?;
        Ok(dao)
    }

    pub async fn load_mut(&self, inode: u64) -> Result<DaoMut<Inode>> {
        let dio = self.dio.trans(self.scope_meta).await;
        let dao = dio.load::<Inode>(&PrimaryKey::from(inode)).await?;
        Ok(dao)
    }

    pub async fn load_mut_io(&self, inode: u64) -> Result<DaoMut<Inode>> {
        let dio = self.dio.trans(self.scope_io).await;
        let dao = dio.load::<Inode>(&PrimaryKey::from(inode)).await?;
        Ok(dao)
    }

    pub async fn create_open_handle(&self, inode: u64, req: &RequestContext, flags: i32) -> Result<OpenHandle>
    {
        let mut writable = false;
        if flags & libc::O_TRUNC != 0 || flags & libc::O_RDWR != 0 || flags & libc::O_WRONLY != 0 {
            self.access_internal(&req, inode, 0o2).await?;
            writable = true;
        }

        if flags & libc::O_RDWR != 0 || flags & libc::O_RDONLY != 0 {
            self.access_internal(&req, inode, 0o4).await?;
        }

        let data = self.load(inode).await?;
        let created = data.when_created();
        let updated = data.when_updated();
        let read_only = flags & libc::O_RDONLY != 0;

        let uid = data.dentry.uid;
        let gid = data.dentry.gid;

        let mut dirty = false;
        
        let mut children = Vec::new();
        if data.kind == FileKind::Directory {
            let fixed = FixedFile::new(data.key().as_u64(), ".".to_string(), FileKind::Directory)
                .uid(uid)
                .gid(gid)
                .created(created)
                .updated(updated);
            children.push(FileSpec::FixedFile(fixed));

            let fixed = FixedFile::new(data.key().as_u64(), "..".to_string(), FileKind::Directory)
                .uid(uid)
                .gid(gid)
                .created(created)
                .updated(updated);
            children.push(FileSpec::FixedFile(fixed));

            match writable {
                true => {
                    let mut data = self.load_mut_io(inode).await?;
                    let mut data = data.as_mut();
                    for child in data.children.iter_mut_ext(true, true).await? {
                        let child_spec = Inode::as_file_spec_mut(child.key().as_u64(), child.when_created(), child.when_updated(), child).await;
                        children.push(child_spec);
                    }
                }
                false => {
                    for child in data.children.iter_ext(true, true).await? {
                        let child_spec = Inode::as_file_spec(child.key().as_u64(), child.when_created(), child.when_updated(), child).await;
                        children.push(child_spec);
                    }
                }
            }
        }
        
        let spec = match writable {
            true => {
                let data = self.load_mut_io(inode).await?;
                Inode::as_file_spec_mut(data.key().as_u64(), created, updated, data).await
            }
            false => Inode::as_file_spec(data.key().as_u64(), created, updated, data).await
        };
        if flags & libc::O_TRUNC != 0 {
            spec.fallocate(0).await?;
            dirty = true;
        }

        let mut open = OpenHandle {
            inode,
            read_only,
            fh: fastrand::u64(..),
            attr: FileAttr::new(&spec, uid, gid),
            kind: spec.kind(),
            spec: spec,
            children: Vec::new(),
            dirty: seqlock::SeqLock::new(dirty),
        };

        for child in children.into_iter() {
            let (uid, gid) = match self.impersonate_uid {
                true => {
                    let uid = self.reverse_uid(child.uid(), req);
                    let gid = self.reverse_gid(child.gid(), req);
                    (uid, gid)
                },
                false => {
                    (child.uid(), child.gid())
                }
            };            
            open.add_child(&child, uid, gid);
        }

        Ok(open)
    }
}

impl FileAccessor
{
    pub async fn dio_mut_io(&self) -> Arc<DioMut>
    {
        let ret = self.dio.trans(self.scope_io).await;
        ret.auto_cancel();
        ret
    }

    pub async fn dio_mut_meta(&self) -> Arc<DioMut>
    {
        let ret = self.dio.trans(self.scope_meta).await;
        ret.auto_cancel();
        ret
    }

    pub async fn mknod_internal(
        &self,
        req: &RequestContext,
        parent: u64,
        name: &str,
        mode: u32,
    ) -> Result<DaoMut<Inode>> {
        
        let key = PrimaryKey::from(parent);
        let dio = self.dio_mut_meta().await;
        let mut data = dio.load::<Inode>(&key).await?;

        if data.kind != FileKind::Directory {
            trace!("create parent={} not-a-directory", parent);
            bail!(FileSystemErrorKind::NotDirectory);
        }
        
        if let Some(_) = data.children.iter().await?.filter(|c| *c.dentry.name == *name).next() {
            trace!("create parent={} name={}: already-exists", parent, name);
            bail!(FileSystemErrorKind::DoesNotExist);
        }

        let uid = self.translate_uid(req.uid, req);
        let gid = self.translate_gid(req.gid, req);
        let child = Inode::new(
            name.to_string(),
            mode, 
            uid,
            gid,
            FileKind::RegularFile,
        );

        let mut child = data.as_mut().children.push(child)?;
        self.update_auth(mode, uid, gid, child.auth_mut())?;
        return Ok(child);
    }

    pub async fn tick(&self) -> Result<()> {
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

    pub async fn commit(&self) -> Result<()> {
        let _ = self.commit_lock.lock();
        self.commit_internal().await?;
        Ok(())
    }

    pub async fn commit_internal(&self) -> Result<()> {
        trace!("commit");
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
            open.spec.commit().await?;
        }
        Ok(())
    }

    pub fn update_auth(&self, mode: u32, uid: u32, gid: u32, mut auth: DaoAuthGuard<'_>) -> Result<()> {
        let inner_key = {
            match &auth.read {
                ReadOption::Inherit => None,
                ReadOption::Everyone(old) => match old.clone() {
                    Some(a) => Some(a),
                    None => {
                        let keysize = match self.get_group_read_key(gid) {
                            Some(a) => a.size(),
                            None => match self.get_user_read_key(uid) {
                                Some(a) => a.size(),
                                None => KeySize::Bit192,
                            }
                        };
                        Some(EncryptKey::generate(keysize))
                    },
                },
                ReadOption::Specific(hash, derived) => {
                    let key = match self.session.read_keys(AteSessionKeyCategory::AllKeys)
                        .filter(|k| k.hash() == *hash)
                        .next()
                    {
                        Some(a) => a.clone(),
                        None => { bail!(FileSystemErrorKind::NoAccess); }
                    };
                    Some(derived.transmute(&key)?)
                }
            }
        };

        if mode & 0o004 != 0 {
            auth.read = ReadOption::Everyone(inner_key);
        } else {
            let new_key = {
                if mode & 0o040 != 0 {
                    self.get_group_read_key(gid)
                } else {
                    self.get_user_read_key(uid)
                }
            };
            if let Some(new_key) = new_key {
                let inner_key = match inner_key {
                    Some(a) => a.clone(),
                    None => EncryptKey::generate(new_key.size())
                };
                auth.read = ReadOption::Specific(new_key.hash(), DerivedEncryptKey::reverse(&new_key, &inner_key));
            } else if self.no_auth == false {
                if mode & 0o040 != 0 {
                    error!("Session does not have the required group ({}) read key embedded within it", gid);
                } else {
                    error!("Session does not have the required user ({}) read key embedded within it", uid);
                }
                debug!("...we have...{}", self.session);
                bail!(FileSystemErrorKind::NoAccess);
            } else {
                auth.read = ReadOption::Inherit;
            }
        }

        if mode & 0o002 != 0 {
            auth.write = WriteOption::Everyone;
        } else {
            let new_key = {
                if mode & 0o020 != 0 {
                    self.get_group_write_key(gid)
                } else {
                    self.get_user_write_key(uid)
                }
            };
            if let Some(key) = new_key {
                auth.write = WriteOption::Specific(key.hash());
            } else if self.no_auth == false {
                if mode & 0o020 != 0 {
                    error!("Session does not have the required group ({}) write key embedded within it", gid);
                } else {
                    error!("Session does not have the required user ({}) write key embedded within it", uid);
                }
                debug!("...we have...{}", self.session);
                bail!(FileSystemErrorKind::NoAccess);
            } else {
                auth.write = WriteOption::Inherit;
            }
        }

        Ok(())
    }

    pub fn translate_uid(&self, uid: u32, req: &RequestContext) -> u32 {
        if uid == 0 || uid == req.uid {
            return self.session.uid().unwrap_or_else(|| uid);
        }
        uid
    }

    pub fn translate_gid(&self, gid: u32, req: &RequestContext) -> u32 {
        if gid == 0 || gid == req.gid {
            return self.session.gid().unwrap_or_else(|| gid);
        }
        gid
    }

    pub fn reverse_uid(&self, uid: u32, req: &RequestContext) -> u32 {
        if self.session.uid() == Some(uid) {
            return req.uid;
        }
        uid
    }

    pub fn reverse_gid(&self, gid: u32, req: &RequestContext) -> u32 {
        if self.session.gid() == Some(gid) {
            return req.gid;
        }
        gid
    }

    pub async fn access_internal(&self, req: &RequestContext, inode: u64, mask: u32) -> Result<()> {
        self.tick().await?;
        trace!("access inode={} mask={:#02x}", inode, mask);
        
        let dao = self.load(inode).await?;
        if (dao.dentry.mode & mask) != 0
        {
            trace!("access mode={:#02x} - ok", dao.dentry.mode);
            return Ok(());
        }

        let uid = self.translate_uid(req.uid, &req);
        if uid == dao.dentry.uid {
            trace!("access has_user");
            let mask_shift = mask << 6;
            if (dao.dentry.mode & mask_shift) != 0
            {
                trace!("access mode={:#02x} - ok", dao.dentry.mode);
                return Ok(());
            }
        }

        let gid = self.translate_gid(req.gid, &req);
        if gid == dao.dentry.gid && self.session.gid() == Some(gid) {
            trace!("access has_group");
            let mask_shift = mask << 3;
            if (dao.dentry.mode & mask_shift) != 0
            {
                trace!("access mode={:#02x} - ok", dao.dentry.mode);
                return Ok(());
            }
        }

        if req.uid == 0 {
            trace!("access mode={:#02x} - ok", dao.dentry.mode);
            return Ok(());
        }

        trace!("access mode={:#02x} - EACCES", dao.dentry.mode);
        bail!(FileSystemErrorKind::NoAccess);
    }
    
    pub async fn getattr(
        &self,
        req: &RequestContext,
        inode: u64,
        fh: Option<u64>,
        _flags: u32,
    ) -> Result<FileAttr> {
        self.tick().await?;
        trace!("getattr inode={}", inode);

        if let Some(fh) = fh {
            let lock = self.open_handles.lock();
            if let Some(open) = lock.get(&fh) {
                return Ok(open.attr.clone())
            }
        }

        let dao = self.load(inode).await?;
        let spec = Inode::as_file_spec(inode, dao.when_created(), dao.when_updated(), dao).await;
        Ok(match self.impersonate_uid {
            true => self.spec_as_attr_reverse(&spec, &req),
            false => FileAttr::new(&spec, spec.uid(), spec.gid())
        })
    }

    pub async fn setattr(
        &self,
        req: &RequestContext,
        inode: u64,
        _fh: Option<u64>,
        set_attr: SetAttr,
    ) -> Result<FileAttr> {
        self.tick().await?;
        trace!("setattr inode={}", inode);

        let key = PrimaryKey::from(inode);
        let dio = self.dio_mut_meta().await;
        let mut dao = dio.load::<Inode>(&key).await?;
        
        let mut changed = false;
        if let Some(uid) = set_attr.uid {
            let new_uid = self.translate_uid(uid, req);
            if dao.dentry.uid != new_uid {
                let mut dao = dao.as_mut();
                dao.dentry.uid = new_uid;
                changed = true;
            }
        }
        if let Some(gid) = set_attr.gid {
            let new_gid = self.translate_gid(gid, req);
            if dao.dentry.gid != new_gid {
                let mut dao = dao.as_mut();
                dao.dentry.gid = new_gid;
                changed = true;
            }
        }
        if let Some(mode) = set_attr.mode {
            if dao.dentry.mode != mode {
                let mut dao = dao.as_mut();
                dao.dentry.mode = mode;
                dao.dentry.uid = self.translate_uid(req.uid, req);
                changed = true;
            }
        }

        if changed == true {
            self.update_auth(dao.dentry.mode, dao.dentry.uid, dao.dentry.gid, dao.auth_mut())?;
            dio.commit().await?;
        }

        let spec = Inode::as_file_spec(inode, dao.when_created(), dao.when_updated(), dao.into()).await;
        Ok(self.spec_as_attr_reverse(&spec, req))
    }

    pub async fn opendir(&self, req: &RequestContext, inode: u64, flags: u32) -> Result<Arc<OpenHandle>> {
        self.tick().await?;
        debug!("atefs::opendir inode={}", inode);

        let open = self.create_open_handle(inode, req, flags as i32).await?;

        if open.attr.kind != FileKind::Directory {
            debug!("atefs::opendir not-a-directory");
            bail!(FileSystemErrorKind::NotDirectory);
        }

        let fh = open.fh;
        let handle = Arc::new(open);
        self.open_handles.lock().insert(fh, Arc::clone(&handle));
        Ok(handle)
    }

    pub async fn releasedir(&self, _req: &RequestContext, inode: u64, fh: u64, _flags: u32) -> Result<()> {
        self.tick().await?;
        debug!("atefs::releasedir inode={}", inode);

        let open = self.open_handles.lock().remove(&fh);
        if let Some(open) = open {
            open.spec.commit().await?
        }
        Ok(())
    }

    pub async fn lookup(&self, req: &RequestContext, parent: u64, name: &str) -> Result<Option<FileAttr>> {
        self.tick().await?;
        let open = self.create_open_handle(parent, req, libc::O_RDONLY).await?;

        if open.attr.kind != FileKind::Directory {
            debug!("atefs::lookup parent={} not-a-directory", parent);
            bail!(FileSystemErrorKind::NotDirectory);
        }
        
        if let Some(entry) = open.children.iter().filter(|c| c.name.as_str() == name).next() {
            debug!("atefs::lookup parent={} name={}: found", parent, name);
            return Ok(Some(entry.attr.clone()));
        }

        debug!("atefs::lookup parent={} name={}: not found", parent, name);
        Ok(None)
    }

    pub async fn search(&self, req: &RequestContext, path: &str) -> Result<Option<FileAttr>> {
        let mut ret = self.getattr(req, 1u64, None, 0u32).await?;
        for comp in path.split("/") {
            if comp.len() <= 0 {
                continue;
            }
            ret = match self.lookup(req, ret.ino, comp).await? {
                Some(a) => a,
                None => {
                    return Ok(None);
                }
            }
        }
        Ok(Some(ret))
    }

    pub async fn forget(&self, _req: &RequestContext, _inode: u64, _nlookup: u64) {
        let _ = self.tick().await;
    }

    pub async fn fsync(&self, _req: &RequestContext, inode: u64, _fh: u64, _datasync: bool) -> Result<()> {
        self.tick().await?;
        debug!("atefs::fsync inode={}", inode);

        Ok(())
    }

    pub async fn flush(&self, _req: &RequestContext, inode: u64, fh: u64, _lock_owner: u64) -> Result<()> {
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
            open.spec.commit().await?
        }

        self.chain.flush().await?;
        Ok(())
    }

    pub async fn access(&self, req: &RequestContext, inode: u64, mask: u32) -> Result<()> {
        self.access_internal(req, inode, mask).await
    }

    pub async fn mkdir(
        &self,
        req: &RequestContext,
        parent: u64,
        name: &str,
        mode: u32,
    ) -> Result<FileAttr> {
        self.tick().await?;
        debug!("atefs::mkdir parent={}", parent);

        let dio = self.dio.trans(self.scope_meta).await;
        let mut data = dio.load::<Inode>(&PrimaryKey::from(parent)).await?;
        
        if data.kind != FileKind::Directory {
            bail!(FileSystemErrorKind::NotDirectory);
        }

        let uid = self.translate_uid(req.uid, req);
        let gid = self.translate_gid(req.gid, req);
        let child = Inode::new(
            name.to_string(),
            mode, 
            uid,
            gid,
            FileKind::Directory,
        );

        let mut child = data.as_mut().children.push(child)?;
        self.update_auth(mode, uid, gid, child.auth_mut())?;
        dio.commit().await?;

        let child_spec = Inode::as_file_spec(child.key().as_u64(), child.when_created(), child.when_updated(), child.into()).await;
        Ok(self.spec_as_attr_reverse(&child_spec, req))
    }

    pub async fn rmdir(&self, req: &RequestContext, parent: u64, name: &str) -> Result<()> {
        self.tick().await?;
        debug!("atefs::rmdir parent={}", parent);

        let open = self.create_open_handle(parent, req, libc::O_RDONLY).await?;

        if open.attr.kind != FileKind::Directory {
            debug!("atefs::rmdir parent={} not-a-directory", parent);
            bail!(FileSystemErrorKind::NotDirectory);
        }
        
        if let Some(entry) = open.children.iter().filter(|c| c.name.as_str() == name).next() {
            debug!("atefs::rmdir parent={} name={}: found", parent, name);

            let dio = self.dio.trans(self.scope_meta).await;
            dio.delete(&PrimaryKey::from(entry.inode)).await?;
            dio.commit().await?;
            return Ok(())
        }

        debug!("atefs::rmdir parent={} name={}: not found", parent, name);
        bail!(FileSystemErrorKind::NoEntry);
    }

    pub async fn interrupt(&self, _req: &RequestContext, unique: u64) -> Result<()> {
        self.tick().await?;
        debug!("atefs::interrupt unique={}", unique);

        Ok(())
    }

    pub async fn mknod(
        &self,
        req: &RequestContext,
        parent: u64,
        name: &str,
        mode: u32,
    ) -> Result<FileAttr> {
        self.tick().await?;
        debug!("atefs::mknod parent={} name={}", parent, name);

        let dao = self.mknod_internal(&req, parent, name, mode).await?;
        dao.trans().commit().await?;

        let spec = Inode::as_file_spec(dao.key().as_u64(), dao.when_created(), dao.when_updated(), dao.into()).await;
        Ok(self.spec_as_attr_reverse(&spec, &req))
    }

    pub async fn create(
        &self,
        req: &RequestContext,
        parent: u64,
        name: &str,
        mode: u32,
    ) -> Result<Arc<OpenHandle>> {
        self.tick().await?;
        debug!("atefs::create parent={} name={}", parent, name);

        let data = self.mknod_internal(req, parent, name, mode).await?;
        data.trans().commit().await?;

        let spec = Inode::as_file_spec_mut(data.key().as_u64(), data.when_created(), data.when_updated(), data.into()).await;
        let attr = self.spec_as_attr_reverse(&spec, req);
        let open = OpenHandle {
            inode: spec.ino(),
            read_only: false,
            fh: fastrand::u64(..),
            kind: spec.kind(),
            spec: spec,
            attr: attr.clone(),
            children: Vec::new(),
            dirty: seqlock::SeqLock::new(false),
        };

        let fh = open.fh;
        let handle = Arc::new(open);
        self.open_handles.lock().insert(fh, Arc::clone(&handle));

        Ok(handle)
    }

    pub async fn unlink(&self, _req: &RequestContext, parent: u64, name: &str) -> Result<()> {
        self.tick().await?;
        debug!("atefs::unlink parent={} name={}", parent, name);

        let parent_key = PrimaryKey::from(parent);
        
        let data_parent = self.dio.load::<Inode>(&parent_key).await?;

        if data_parent.kind != FileKind::Directory {
            debug!("atefs::unlink parent={} not-a-directory", parent);
            bail!(FileSystemErrorKind::NotDirectory);
        }
        
        if let Some(data) = data_parent.children.iter().await?.filter(|c| c.dentry.name.as_str() == name).next()
        {
            if data.kind == FileKind::Directory {
                debug!("atefs::unlink parent={} name={} is-a-directory", parent, name);
                bail!(FileSystemErrorKind::IsDirectory);
            }

            let dio = self.dio_mut_meta().await;
            dio.delete(&data.key()).await?;
            dio.commit().await?;

            return Ok(());
        }
        bail!(FileSystemErrorKind::NoEntry);
    }

    pub async fn rename(
        &self,
        _req: &RequestContext,
        parent: u64,
        name: &str,
        new_parent: u64,
        new_name: &str,
    ) -> Result<()> {
        self.tick().await?;
        debug!("atefs::rename name={} new_name={}", name, new_name);
        
        let mut parent_data = self.load_mut(parent).await?;
        if parent_data.kind != FileKind::Directory {
            debug!("atefs::rename parent={} not-a-directory", parent);
            bail!(FileSystemErrorKind::NotDirectory);
        }
        
        let dio = parent_data.trans();
        let mut parent_data = parent_data.as_mut();
        if let Some(mut data) = parent_data.children.iter_mut().await?.filter(|c| c.dentry.name.as_str() == name).next()
        {
            // If the parent has changed then move it
            if parent != new_parent
            {
                let new_parent_key = PrimaryKey::from(new_parent);
                let new_parent_data = self.dio.load::<Inode>(&new_parent_key).await?;
                
                if new_parent_data.kind != FileKind::Directory {
                    debug!("atefs::rename new_parent={} not-a-directory", new_parent);
                    bail!(FileSystemErrorKind::NotDirectory);
                }

                if new_parent_data.children.iter().await?.filter(|c| c.dentry.name.as_str() == new_name).next().is_some() {
                    debug!("atefs::rename new_name={} already exists", new_name);
                    bail!(FileSystemErrorKind::AlreadyExists);
                }

                data.detach()?;
                data.attach(&new_parent_data, &new_parent_data.children)?;
            }
            else
            {
                if parent_data.children.iter().await?.filter(|c| c.dentry.name.as_str() == new_name).next().is_some() {
                    debug!("atefs::rename new_name={} already exists", new_name);
                    bail!(FileSystemErrorKind::NotDirectory);
                }
            }

            data.as_mut().dentry.name = new_name.to_string();
            drop(parent_data);
            
            dio.commit().await?;
            return Ok(());
        }
        bail!(FileSystemErrorKind::NoEntry);
    }

    pub async fn open(&self, req: &RequestContext, inode: u64, flags: u32) -> Result<Arc<OpenHandle>> {
        self.tick().await?;
        debug!("atefs::open inode={}", inode);

        let open = self.create_open_handle(inode, &req, flags as i32).await?;
        if open.kind == FileKind::Directory {
            debug!("atefs::open is-a-directory");
            bail!(FileSystemErrorKind::IsDirectory);
        }

        let fh = open.fh;
        let handle = Arc::new(open);
        self.open_handles.lock().insert(fh, Arc::clone(&handle));
        Ok(handle)
    }

    pub async fn release(
        &self,
        _req: &RequestContext,
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
            open.spec.commit().await?
        }

        if flush {
            self.chain.flush().await?;
        }

        Ok(())
    }

    pub async fn read(
        &self,
        _req: &RequestContext,
        inode: u64,
        fh: u64,
        offset: u64,
        size: u32,
    ) -> Result<Bytes> {
        self.tick().await?;
        debug!("atefs::read inode={} offset={} size={}", inode, offset, size);
        
        let open = {
            let lock = self.open_handles.lock();
            match lock.get(&fh) {
                Some(a) => Arc::clone(a),
                None => {
                    bail!(FileSystemErrorKind::NotImplemented);
                },
            }
        };
        open.spec.read(offset, size as u64).await
    }

    pub async fn write(
        &self,
        _req: &RequestContext,
        inode: u64,
        fh: u64,
        offset: u64,
        data: &[u8],
        _flags: u32,
    ) -> Result<u64> {
        self.tick().await?;
        debug!("atefs::write inode={} offset={} size={}", inode, offset, data.len());

        let open = {
            let lock = self.open_handles.lock();
            match lock.get(&fh) {
                Some(a) => Arc::clone(a),
                None => {
                    debug!("atefs::write-failed inode={} offset={} size={}", inode, offset, data.len());
                    bail!(FileSystemErrorKind::NotImplemented);
                },
            }
        };

        if open.read_only {
            bail!(FileSystemErrorKind::ReadOnly);
        }

        let wrote = open.spec.write(offset, data).await?;
        if open.dirty.read() == false {
            *open.dirty.lock_write() = true;
        }

        debug!("atefs::wrote inode={} offset={} size={}", inode, offset, wrote);
        Ok(wrote)
    }

    pub async fn fallocate(
        &self,
        _req: &RequestContext,
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
                if open.read_only {
                    bail!(FileSystemErrorKind::ReadOnly);
                }
                
                open.spec.fallocate(offset + length).await?;
                if open.dirty.read() == false {
                    *open.dirty.lock_write() = true;
                }
                return Ok(());
            }
        }

        let mut dao = self.load_mut(inode).await?;
        dao.as_mut().size = offset + length;
        dao.trans().commit().await?;

        return Ok(());
    }

    pub async fn lseek(
        &self,
        _req: &RequestContext,
        inode: u64,
        fh: u64,
        offset: u64,
        whence: u32,
    ) -> Result<u64> {
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
                None => self.load(inode).await?.size
            };
            offset + size
        } else {
            bail!(FileSystemErrorKind::InvalidArguments);
        };
        Ok(offset)
    }

    pub async fn symlink(
        &self,
        req: &RequestContext,
        parent: u64,
        name: &str,
        link: &str,
    ) -> Result<FileAttr> {
        self.tick().await?;
        debug!("atefs::symlink parent={}, name={}, link={}", parent, name, link);

        let link = link.to_string();
        let spec = {
            let mut dao = self.mknod_internal(req, parent, name, 0o770).await?;
            {
                let mut dao = dao.as_mut();
                dao.kind = FileKind::SymLink;
                dao.link = Some(link);
            }
            dao.trans().commit().await?;

            Inode::as_file_spec(dao.key().as_u64(), dao.when_created(), dao.when_updated(), dao.into()).await
        };
        
        Ok(self.spec_as_attr_reverse(&spec, &req))
    }
    
    pub async fn setxattr(
        &self,
        req: &RequestContext,
        inode: u64,
        name: &str,
        value: &str,
    ) -> Result<()> {
        self.tick().await?;
        
        let flags = libc::O_RDWR;
        let mut open = self.create_open_handle(inode, &req, flags).await?;
        open.spec.set_xattr(name, value).await
    }

    /// remove an extended attribute.
    pub async fn removexattr(
        &self,
        req: &RequestContext,
        inode: u64,
        name: &str
    ) -> Result<bool> {
        self.tick().await?;
        debug!("atefs::removexattr not-implemented");

        let flags = libc::O_RDWR;
        let mut open = self.create_open_handle(inode, &req, flags).await?;
        open.spec.remove_xattr(name).await
    }
    
    pub async fn getxattr(
        &self,
        req: &RequestContext,
        inode: u64,
        name: &str,
    ) -> Result<Option<String>> {
        self.tick().await?;
        
        let flags = libc::O_RDONLY;
        let open = self.create_open_handle(inode, &req, flags).await?;
        open.spec.get_xattr(name).await
    }
    
    pub async fn listxattr(
        &self,
        req: &RequestContext,
        inode: u64,
    ) -> Result<FxHashMap<String, String>> {
        self.tick().await?;
        debug!("atefs::listxattr not-implemented");

        let flags = libc::O_RDONLY;
        let open = self.create_open_handle(inode, &req, flags).await?;
        open.spec.list_xattr().await
    }
}