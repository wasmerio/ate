#![allow(unused_imports)]
use ate_files::accessor::FileAccessor;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use parking_lot::Mutex;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::vec::IntoIter;

use ::ate::chain::*;
use ::ate::crypto::*;
use ::ate::dio::Dao;
use ::ate::dio::DaoObj;
use ::ate::dio::Dio;
use ::ate::error::*;
use ::ate::header::PrimaryKey;
use ::ate::prelude::AteRolePurpose;
use ::ate::prelude::ReadOption;
use ::ate::prelude::*;
use ::ate::session::AteSessionUser;
use ::ate::{crypto::DerivedEncryptKey, prelude::TransactionScope};

use ate_files::prelude::*;

use async_trait::async_trait;
use futures_util::stream;
use futures_util::stream::Iter;
use fxhash::FxHashMap;

const FUSE_TTL: Duration = Duration::from_secs(1);

use ate_files::model;

use super::error::conv_result;
use super::fuse;

pub struct AteFS
where
    Self: Send + Sync,
{
    accessor: FileAccessor,
    umask: u32,
}

pub fn conv_attr(attr: &FileAttr) -> fuse::FileAttr {
    let size = attr.size;
    let blksize = model::PAGE_SIZE as u64;

    fuse::FileAttr {
        ino: attr.ino,
        generation: 0,
        size: attr.size,
        blocks: (size / blksize),
        atime: SystemTime::UNIX_EPOCH + Duration::from_millis(attr.accessed),
        mtime: SystemTime::UNIX_EPOCH + Duration::from_millis(attr.updated),
        ctime: SystemTime::UNIX_EPOCH + Duration::from_millis(attr.created),
        kind: conv_kind(attr.kind),
        perm: fuse3::perm_from_mode_and_kind(conv_kind(attr.kind), attr.mode),
        nlink: 0,
        uid: attr.uid,
        gid: attr.gid,
        rdev: 0,
        blksize: blksize as u32,
    }
}

fn conv_kind(kind: FileKind) -> fuse::FileType {
    match kind {
        FileKind::Directory => fuse::FileType::Directory,
        FileKind::FixedFile => fuse::FileType::RegularFile,
        FileKind::RegularFile => fuse::FileType::RegularFile,
        FileKind::SymLink => fuse::FileType::Symlink,
    }
}

impl AteFS {
    pub async fn new(
        chain: Arc<Chain>,
        group: Option<String>,
        session: AteSessionType,
        scope_io: TransactionScope,
        scope_meta: TransactionScope,
        no_auth: bool,
        impersonate_uid: bool,
        umask: u32,
    ) -> AteFS {
        AteFS {
            accessor: FileAccessor::new(
                chain,
                group,
                session,
                scope_io,
                scope_meta,
                no_auth,
                impersonate_uid,
            )
            .await,
            umask,
        }
    }

    pub async fn load(&self, inode: u64) -> fuse::Result<Dao<Inode>> {
        conv_result(self.accessor.load(inode).await)
    }

    pub async fn load_mut(&self, inode: u64) -> fuse::Result<DaoMut<Inode>> {
        conv_result(self.accessor.load_mut(inode).await)
    }

    pub async fn load_mut_io(&self, inode: u64) -> fuse::Result<DaoMut<Inode>> {
        conv_result(self.accessor.load_mut_io(inode).await)
    }

    async fn create_open_handle(
        &self,
        inode: u64,
        req: &fuse::Request,
        flags: i32,
    ) -> fuse::Result<OpenHandle> {
        let req = req_ctx(req);
        conv_result(self.accessor.create_open_handle(inode, &req, flags).await)
    }
}

impl AteFS {
    async fn tick(&self) -> fuse::Result<()> {
        conv_result(self.accessor.tick().await)
    }
}

fn req_ctx(req: &fuse::Request) -> RequestContext {
    RequestContext {
        uid: req.uid,
        gid: req.gid,
    }
}

#[async_trait]
impl fuse::Filesystem for AteFS {
    type DirEntryStream = Iter<IntoIter<fuse::Result<fuse3::raw::prelude::DirectoryEntry>>>;
    type DirEntryPlusStream = Iter<IntoIter<fuse::Result<fuse3::raw::prelude::DirectoryEntryPlus>>>;

    async fn init(&self, req: fuse::Request) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(self.accessor.init(&req).await)
    }

    async fn destroy(&self, req: fuse::Request) {
        let _req = req_ctx(&req);
    }

    async fn getattr(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: Option<u64>,
        flags: u32,
    ) -> fuse::Result<fuse::ReplyAttr> {
        let req = req_ctx(&req);
        Ok(fuse::ReplyAttr {
            ttl: FUSE_TTL,
            attr: conv_attr(&conv_result(
                self.accessor.getattr(&req, inode, fh, flags).await,
            )?),
        })
    }

    async fn setattr(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: Option<u64>,
        set_attr: fuse::SetAttr,
    ) -> fuse::Result<fuse::ReplyAttr> {
        let req = req_ctx(&req);

        let set_attr = SetAttr {
            mode: set_attr.mode,
            uid: set_attr.uid,
            gid: set_attr.gid,
            size: set_attr.size,
            lock_owner: set_attr.lock_owner,
            accessed: set_attr
                .atime
                .iter()
                .filter_map(|a| {
                    a.duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                        .map(|a| a.as_millis() as u64)
                })
                .next(),
            updated: set_attr
                .mtime
                .iter()
                .filter_map(|a| {
                    a.duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                        .map(|a| a.as_millis() as u64)
                })
                .next(),
            created: set_attr
                .ctime
                .iter()
                .filter_map(|a| {
                    a.duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                        .map(|a| a.as_millis() as u64)
                })
                .next(),
        };
        let attr = conv_attr(&conv_result(
            self.accessor.setattr(&req, inode, fh, set_attr).await,
        )?);
        Ok(fuse::ReplyAttr {
            ttl: FUSE_TTL,
            attr,
        })
    }

    async fn opendir(
        &self,
        req: fuse::Request,
        inode: u64,
        flags: u32,
    ) -> fuse::Result<fuse::ReplyOpen> {
        let req = req_ctx(&req);
        Ok(fuse::ReplyOpen {
            fh: conv_result(self.accessor.opendir(&req, inode, flags).await)?.fh,
            flags: 0,
        })
    }

    async fn releasedir(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        flags: u32,
    ) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(self.accessor.releasedir(&req, inode, fh, flags).await)
    }

    async fn readdirplus(
        &self,
        req: fuse::Request,
        parent: u64,
        fh: u64,
        offset: u64,
        _lock_owner: u64,
    ) -> fuse::Result<fuse::ReplyDirectoryPlus<Self::DirEntryPlusStream>> {
        self.tick().await?;
        debug!("atefs::readdirplus id={} offset={}", parent, offset);

        if fh == 0 {
            let open = self
                .create_open_handle(parent, &req, libc::O_RDONLY)
                .await?;
            let entries = open
                .children
                .iter()
                .skip(offset as usize)
                .map(|a| {
                    Ok(fuse::DirectoryEntryPlus {
                        inode: a.inode,
                        kind: conv_kind(a.kind),
                        name: OsString::from(a.name.as_str()),
                        generation: 0,
                        attr: conv_attr(&a.attr),
                        entry_ttl: FUSE_TTL,
                        attr_ttl: FUSE_TTL,
                    })
                })
                .map(|a| conv_result(a))
                .collect::<Vec<_>>();
            return Ok(fuse::ReplyDirectoryPlus {
                entries: stream::iter(entries.into_iter()),
            });
        }

        let lock = self.accessor.open_handles.lock();
        if let Some(open) = lock.get(&fh) {
            let entries = open
                .children
                .iter()
                .skip(offset as usize)
                .map(|a| {
                    Ok(fuse::DirectoryEntryPlus {
                        inode: a.inode,
                        kind: conv_kind(a.kind),
                        name: OsString::from(a.name.as_str()),
                        generation: 0,
                        attr: conv_attr(&a.attr),
                        entry_ttl: FUSE_TTL,
                        attr_ttl: FUSE_TTL,
                    })
                })
                .map(|a| conv_result(a))
                .collect::<Vec<_>>();
            Ok(fuse::ReplyDirectoryPlus {
                entries: stream::iter(entries.into_iter()),
            })
        } else {
            Err(libc::ENOSYS.into())
        }
    }

    async fn readdir(
        &self,
        req: fuse::Request,
        parent: u64,
        fh: u64,
        offset: i64,
    ) -> fuse::Result<fuse::ReplyDirectory<Self::DirEntryStream>> {
        self.tick().await?;
        debug!("atefs::readdir parent={}", parent);

        if fh == 0 {
            let open = self
                .create_open_handle(parent, &req, libc::O_RDONLY)
                .await?;
            let entries = open
                .children
                .iter()
                .skip(offset as usize)
                .map(|a| {
                    Ok(fuse::DirectoryEntry {
                        inode: a.inode,
                        kind: conv_kind(a.kind),
                        name: OsString::from(a.name.as_str()),
                    })
                })
                .map(|a| conv_result(a))
                .collect::<Vec<_>>();
            return Ok(fuse::ReplyDirectory {
                entries: stream::iter(entries.into_iter()),
            });
        }

        let lock = self.accessor.open_handles.lock();
        if let Some(open) = lock.get(&fh) {
            let entries = open
                .children
                .iter()
                .skip(offset as usize)
                .map(|a| {
                    Ok(fuse::DirectoryEntry {
                        inode: a.inode,
                        kind: conv_kind(a.kind),
                        name: OsString::from(a.name.as_str()),
                    })
                })
                .map(|a| conv_result(a))
                .collect::<Vec<_>>();
            Ok(fuse::ReplyDirectory {
                entries: stream::iter(entries.into_iter()),
            })
        } else {
            Err(libc::ENOSYS.into())
        }
    }

    async fn lookup(
        &self,
        req: fuse::Request,
        parent: u64,
        name: &OsStr,
    ) -> fuse::Result<fuse::ReplyEntry> {
        let req = req_ctx(&req);
        let name = name.to_str().unwrap();
        Ok(fuse::ReplyEntry {
            ttl: FUSE_TTL,
            attr: match conv_result(self.accessor.lookup(&req, parent, name).await)? {
                Some(a) => conv_attr(&a),
                None => {
                    return Err(libc::ENOENT.into());
                }
            },
            generation: 0,
        })
    }

    async fn forget(&self, req: fuse::Request, inode: u64, nlookup: u64) {
        let req = req_ctx(&req);
        self.accessor.forget(&req, inode, nlookup).await;
    }

    async fn fsync(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        datasync: bool,
    ) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(self.accessor.fsync(&req, inode, fh, datasync).await)
    }

    async fn flush(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        lock_owner: u64,
    ) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(self.accessor.flush(&req, inode, fh, lock_owner).await)
    }

    async fn access(&self, req: fuse::Request, inode: u64, mask: u32) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(self.accessor.access(&req, inode, mask).await)
    }

    async fn mkdir(
        &self,
        req: fuse::Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
    ) -> fuse::Result<fuse::ReplyEntry> {
        let req = req_ctx(&req);
        let mode = if self.umask != 0o0000 {
            0o777 & !self.umask
        } else {
            mode
        };
        let attr = conv_result(
            self.accessor
                .mkdir(&req, parent, name.to_str().unwrap(), mode)
                .await,
        )?;
        Ok(fuse::ReplyEntry {
            ttl: FUSE_TTL,
            attr: conv_attr(&attr),
            generation: 0,
        })
    }

    async fn rmdir(&self, req: fuse::Request, parent: u64, name: &OsStr) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(
            self.accessor
                .rmdir(&req, parent, name.to_str().unwrap())
                .await,
        )
    }

    async fn interrupt(&self, req: fuse::Request, unique: u64) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(self.accessor.interrupt(&req, unique).await)
    }

    async fn mknod(
        &self,
        req: fuse::Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _rdev: u32,
    ) -> fuse::Result<fuse::ReplyEntry> {
        let req = req_ctx(&req);
        let mode = if self.umask != 0o0000 {
            0o666 & !self.umask
        } else {
            mode
        };
        let node = self
            .accessor
            .mknod(&req, parent, name.to_str().unwrap(), mode)
            .await;
        Ok(fuse::ReplyEntry {
            ttl: FUSE_TTL,
            attr: conv_attr(&conv_result(node)?),
            generation: 0,
        })
    }

    async fn create(
        &self,
        req: fuse::Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> fuse::Result<fuse::ReplyCreated> {
        let req = req_ctx(&req);
        let mode = if self.umask != 0o0000 {
            0o666 & !self.umask
        } else {
            mode
        };
        let handle = conv_result(
            self.accessor
                .create(&req, parent, name.to_str().unwrap(), mode)
                .await,
        )?;
        Ok(fuse::ReplyCreated {
            ttl: FUSE_TTL,
            attr: conv_attr(&handle.attr),
            generation: 0,
            fh: handle.fh,
            flags,
        })
    }

    async fn unlink(&self, req: fuse::Request, parent: u64, name: &OsStr) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(
            self.accessor
                .unlink(&req, parent, name.to_str().unwrap())
                .await,
        )
    }

    async fn rename(
        &self,
        req: fuse::Request,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: &OsStr,
    ) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(
            self.accessor
                .rename(
                    &req,
                    parent,
                    name.to_str().unwrap(),
                    new_parent,
                    new_name.to_str().unwrap(),
                )
                .await,
        )
    }

    async fn open(
        &self,
        req: fuse::Request,
        inode: u64,
        flags: u32,
    ) -> fuse::Result<fuse::ReplyOpen> {
        let req = req_ctx(&req);
        let handle = conv_result(self.accessor.open(&req, inode, flags).await)?;
        Ok(fuse::ReplyOpen {
            fh: handle.fh,
            flags,
        })
    }

    async fn release(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        flags: u32,
        lock_owner: u64,
        flush: bool,
    ) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(
            self.accessor
                .release(&req, inode, fh, flags, lock_owner, flush)
                .await,
        )
    }

    async fn read(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        offset: u64,
        size: u32,
    ) -> fuse::Result<fuse::ReplyData> {
        let req = req_ctx(&req);
        let data = conv_result(self.accessor.read(&req, inode, fh, offset, size).await)?;
        Ok(fuse::ReplyData { data })
    }

    async fn write(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        offset: u64,
        data: &[u8],
        flags: u32,
    ) -> fuse::Result<fuse::ReplyWrite> {
        let req = req_ctx(&req);
        let wrote = conv_result(
            self.accessor
                .write(&req, inode, fh, offset, data, flags)
                .await,
        )?;
        Ok(fuse::ReplyWrite { written: wrote })
    }

    async fn fallocate(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        offset: u64,
        length: u64,
        mode: u32,
    ) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(
            self.accessor
                .fallocate(&req, inode, fh, offset, length, mode)
                .await,
        )
    }

    async fn lseek(
        &self,
        req: fuse::Request,
        inode: u64,
        fh: u64,
        offset: u64,
        whence: u32,
    ) -> fuse::Result<fuse::ReplyLSeek> {
        let req = req_ctx(&req);
        let offset = conv_result(self.accessor.lseek(&req, inode, fh, offset, whence).await)?;
        Ok(fuse::ReplyLSeek { offset })
    }

    async fn symlink(
        &self,
        req: fuse::Request,
        parent: u64,
        name: &OsStr,
        link: &OsStr,
    ) -> fuse::Result<fuse::ReplyEntry> {
        let req = req_ctx(&req);
        let attr = conv_result(
            self.accessor
                .symlink(&req, parent, name.to_str().unwrap(), link.to_str().unwrap())
                .await,
        )?;
        Ok(fuse::ReplyEntry {
            ttl: FUSE_TTL,
            attr: conv_attr(&attr),
            generation: 0,
        })
    }

    /// read symbolic link.
    async fn readlink(&self, req: fuse::Request, _inode: u64) -> fuse::Result<fuse::ReplyData> {
        let _req = req_ctx(&req);
        Err(libc::ENOSYS.into())
    }

    /// create a hard link.
    async fn link(
        &self,
        req: fuse::Request,
        _inode: u64,
        _new_parent: u64,
        _new_name: &OsStr,
    ) -> fuse::Result<fuse::ReplyEntry> {
        let _req = req_ctx(&req);
        Err(libc::ENOSYS.into())
    }

    /// get filesystem statistics.
    async fn statsfs(&self, req: fuse::Request, _inode: u64) -> fuse::Result<fuse::ReplyStatFs> {
        let _req = req_ctx(&req);
        Err(libc::ENOSYS.into())
    }

    /// set an extended attribute.
    async fn setxattr(
        &self,
        req: fuse::Request,
        inode: u64,
        name: &OsStr,
        value: &OsStr,
        _flags: u32,
        _position: u32,
    ) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(
            self.accessor
                .setxattr(&req, inode, name.to_str().unwrap(), value.to_str().unwrap())
                .await,
        )
    }

    /// get an extended attribute. If size is too small, use [`ReplyXAttr::Size`] to return correct
    /// size. If size is enough, use [`ReplyXAttr::Data`] to send it, or return error.
    async fn getxattr(
        &self,
        req: fuse::Request,
        inode: u64,
        name: &OsStr,
        size: u32,
    ) -> fuse::Result<fuse::ReplyXAttr> {
        let req = req_ctx(&req);
        let ret = match conv_result(
            self.accessor
                .getxattr(&req, inode, name.to_str().unwrap())
                .await,
        )? {
            Some(a) => a,
            None => {
                return Err(libc::ENODATA.into());
            }
        };

        let ret = {
            let mut r = ret;
            r.push('\0');
            r
        };
        let ret = ret.into_bytes();
        if ret.len() as u32 > size {
            Ok(fuse::ReplyXAttr::Size(ret.len() as u32))
        } else {
            Ok(fuse::ReplyXAttr::Data(bytes::Bytes::from(ret)))
        }
    }

    /// list extended attribute names. If size is too small, use [`ReplyXAttr::Size`] to return
    /// correct size. If size is enough, use [`ReplyXAttr::Data`] to send it, or return error.
    async fn listxattr(
        &self,
        req: fuse::Request,
        inode: u64,
        size: u32,
    ) -> fuse::Result<fuse::ReplyXAttr> {
        let req = req_ctx(&req);
        let attr = conv_result(self.accessor.listxattr(&req, inode).await)?;
        let mut ret = String::new();
        for (k, _) in attr {
            ret.push_str(k.as_str());
            ret.push('\0');
        }
        let ret = ret.into_bytes();
        if ret.len() as u32 > size {
            Ok(fuse::ReplyXAttr::Size(ret.len() as u32))
        } else {
            Ok(fuse::ReplyXAttr::Data(bytes::Bytes::from(ret)))
        }
    }

    /// remove an extended attribute.
    async fn removexattr(&self, req: fuse::Request, inode: u64, name: &OsStr) -> fuse::Result<()> {
        let req = req_ctx(&req);
        conv_result(
            self.accessor
                .removexattr(&req, inode, name.to_str().unwrap())
                .await,
        )?;
        Ok(())
    }

    /// map block index within file to block index within device.
    ///
    /// # Notes:
    ///
    /// This may not works because currently this crate doesn't support fuseblk mode yet.
    async fn bmap(
        &self,
        req: fuse::Request,
        _inode: u64,
        _blocksize: u32,
        _idx: u64,
    ) -> fuse::Result<fuse::ReplyBmap> {
        let _req = req_ctx(&req);
        Err(libc::ENOSYS.into())
    }

    async fn poll(
        &self,
        req: fuse::Request,
        _inode: u64,
        _fh: u64,
        _kh: Option<u64>,
        _flags: u32,
        _events: u32,
        _notify: &fuse::Notify,
    ) -> fuse::Result<fuse::ReplyPoll> {
        let _req = req_ctx(&req);
        Err(libc::ENOSYS.into())
    }

    async fn notify_reply(
        &self,
        req: fuse::Request,
        _inode: u64,
        _offset: u64,
        _data: bytes::Bytes,
    ) -> fuse::Result<()> {
        let _req = req_ctx(&req);
        Err(libc::ENOSYS.into())
    }

    /// forget more than one inode. This is a batch version [`forget`][Filesystem::forget]
    async fn batch_forget(&self, req: fuse::Request, _inodes: &[u64]) {
        let _req = req_ctx(&req);
    }

    async fn copy_file_range(
        &self,
        req: fuse::Request,
        _inode: u64,
        _fh_in: u64,
        _off_in: u64,
        _inode_out: u64,
        _fh_out: u64,
        _off_out: u64,
        _length: u64,
        _flags: u64,
    ) -> fuse::Result<fuse::ReplyCopyFileRange> {
        let _req = req_ctx(&req);
        Err(libc::ENOSYS.into())
    }
}
