#![allow(dead_code)]
use std::io;
use std::path::Path;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api;

pub use crate::api::Dir;
pub use crate::api::FsError;
pub use crate::api::FsResult;
pub use crate::api::Metadata;

#[derive(Clone)]
pub struct FileSystem {
    fs: Arc<dyn api::FileSystem>,
}

impl FileSystem {
    pub async fn mount(wapm: &str, name: &str) -> FsResult<FileSystem> {
        let fs = api::FuseClient::new(wapm)
            .mount(name.to_string())
            .await
            .map_err(|_| FsError::IOError)?;
        let _ = fs.init().await;
        Ok(FileSystem { fs })
    }

    pub async fn mount_with_session(wapm: &str, session: &str, name: &str) -> FsResult<FileSystem> {
        let fs = api::FuseClient::new_with_session(wapm, session)
            .mount(name.to_string())
            .await
            .map_err(|_| FsError::IOError)?;
        let _ = fs.init().await;
        Ok(FileSystem { fs })
    }

    pub async fn read_dir(&self, path: &Path) -> FsResult<Dir> {
        debug!("read_dir: path={}", path.display());

        self.fs
            .read_dir(path.to_string_lossy().to_string())
            .await
            .map_err(|_| FsError::IOError)?
    }

    pub async fn create_dir(&self, path: &Path) -> FsResult<Metadata> {
        debug!("create_dir: path={}", path.display());

        self.fs
            .create_dir(path.to_string_lossy().to_string())
            .await
            .map_err(|_| FsError::IOError)?
    }

    pub async fn remove_dir(&self, path: &Path) -> FsResult<()> {
        debug!("remove_dir: path={}", path.display());

        self.fs
            .remove_dir(path.to_string_lossy().to_string())
            .await
            .map_err(|_| FsError::IOError)?
    }

    pub async fn rename(&self, from: &Path, to: &Path) -> FsResult<()> {
        debug!("rename: from={}, to={}", from.display(), to.display());

        self.fs
            .rename(
                from.to_string_lossy().to_string(),
                to.to_string_lossy().to_string(),
            )
            .await
            .map_err(|_| FsError::IOError)?
    }

    pub async fn metadata(&self, path: &Path) -> FsResult<Metadata> {
        debug!("metadata: path={}", path.display());

        self.fs
            .read_metadata(path.to_string_lossy().to_string())
            .await
            .map_err(|_| FsError::IOError)?
    }

    pub async fn symlink_metadata(&self, path: &Path) -> FsResult<Metadata> {
        debug!("symlink_metadata: path={}", path.display());

        self.fs
            .read_symlink_metadata(path.to_string_lossy().to_string())
            .await
            .map_err(|_| FsError::IOError)?
    }

    pub async fn remove_file(&self, path: &Path) -> FsResult<()> {
        debug!("remove_file: path={}", path.display());

        self.fs
            .remove_file(path.to_string_lossy().to_string())
            .await
            .map_err(|_| FsError::IOError)?
    }

    pub fn new_open_options(&self) -> OpenOptions {
        OpenOptions::new(self.clone())
    }
}

pub struct OpenOptionsConfig {
    read: bool,
    write: bool,
    create_new: bool,
    create: bool,
    append: bool,
    truncate: bool,
}

pub struct OpenOptions {
    fs: FileSystem,
    conf: OpenOptionsConfig,
}

impl OpenOptions {
    pub fn new(fs: FileSystem) -> Self {
        Self {
            fs,
            conf: OpenOptionsConfig {
                read: false,
                write: false,
                create_new: false,
                create: false,
                append: false,
                truncate: false,
            },
        }
    }

    pub fn set_options(&mut self, options: OpenOptionsConfig) -> &mut Self {
        self.conf = options;
        self
    }

    pub fn read(&mut self, read: bool) -> &mut Self {
        self.conf.read = read;
        self
    }

    pub fn write(&mut self, write: bool) -> &mut Self {
        self.conf.write = write;
        self
    }

    pub fn append(&mut self, append: bool) -> &mut Self {
        self.conf.append = append;
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.conf.truncate = truncate;
        self
    }

    pub fn create(&mut self, create: bool) -> &mut Self {
        self.conf.create = create;
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.conf.create_new = create_new;
        self
    }

    pub async fn open(&mut self, path: &Path) -> FsResult<VirtualFile> {
        debug!("open: path={}", path.display());

        let fd = self
            .fs
            .fs
            .open(
                path.to_string_lossy().to_string(),
                api::OpenOptions {
                    read: self.conf.read,
                    write: self.conf.write,
                    create_new: self.conf.create_new,
                    create: self.conf.create,
                    append: self.conf.append,
                    truncate: self.conf.truncate,
                },
            )
            .await
            .map_err(|_| FsError::IOError)?;

        let meta = fd.meta().await.map_err(|_| FsError::IOError)??;

        Ok(VirtualFile {
            io: fd.io().await.map_err(|_| FsError::IOError)?,
            fs: self.fs.clone(),
            fd,
            meta,
        })
    }
}

pub struct VirtualFile {
    fs: FileSystem,
    fd: Arc<dyn api::OpenedFile>,
    io: Arc<dyn api::FileIO>,
    meta: Metadata,
}

impl io::Seek for VirtualFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            io::SeekFrom::Current(a) => api::SeekFrom::Current(a),
            io::SeekFrom::End(a) => api::SeekFrom::End(a),
            io::SeekFrom::Start(a) => api::SeekFrom::Start(a),
        };

        self.io
            .blocking_seek(seek)
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }
}

impl io::Write for VirtualFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io
            .blocking_write(buf.to_vec())
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
            .map(|a| a as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.io
            .blocking_flush()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }
}

impl io::Read for VirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data: Result<_, io::Error> = self
            .io
            .blocking_read(buf.len() as u64)
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());

        let data = data?;
        if data.len() <= 0 {
            return Ok(0usize);
        }

        let dst = &mut buf[..data.len()];
        dst.copy_from_slice(&data[..]);
        Ok(data.len())
    }
}

impl VirtualFile {
    pub fn last_accessed(&self) -> u64 {
        self.meta.accessed
    }

    pub fn last_modified(&self) -> u64 {
        self.meta.modified
    }

    pub fn created_time(&self) -> u64 {
        self.meta.created
    }

    pub fn size(&self) -> u64 {
        self.meta.len
    }

    pub async fn set_len(&mut self, new_size: u64) -> FsResult<()> {
        let result: FsResult<()> = self
            .fd
            .set_len(new_size)
            .await
            .map_err(|_| FsError::IOError)?;
        result?;

        self.meta.len = new_size;
        Ok(())
    }

    pub async fn unlink(&mut self) -> FsResult<()> {
        self.fd.unlink().await.map_err(|_| FsError::IOError)?
    }

    pub async fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            io::SeekFrom::Current(a) => api::SeekFrom::Current(a),
            io::SeekFrom::End(a) => api::SeekFrom::End(a),
            io::SeekFrom::Start(a) => api::SeekFrom::Start(a),
        };

        self.io
            .seek(seek)
            .await
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }

    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io
            .write(buf.to_vec())
            .await
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
            .map(|a| a as usize)
    }

    pub async fn flush(&mut self) -> io::Result<()> {
        self.io
            .flush()
            .await
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data: Result<_, io::Error> = self
            .io
            .read(buf.len() as u64)
            .await
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());

        let data = data?;
        if data.len() <= 0 {
            return Ok(0usize);
        }

        let dst = &mut buf[..data.len()];
        dst.copy_from_slice(&data[..]);
        Ok(data.len())
    }
}
