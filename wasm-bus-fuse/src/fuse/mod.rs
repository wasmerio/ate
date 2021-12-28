#![allow(dead_code)]
use std::io;
use std::path::Path;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api;

pub use crate::api::Dir;
pub use crate::api::FsError;
pub use crate::api::FsResult;
pub use crate::api::Metadata;

#[derive(Debug, Clone)]
pub struct FileSystem {
    fs: api::FileSystem,
}

impl FileSystem {
    pub async fn mount(wapm: &str, name: &str) -> FileSystem {
        let fs = api::Fuse::mount(wapm, name.to_string());
        let _ = fs.init().join().await;
        FileSystem { fs }
    }

    pub async fn mount_with_session(wapm: &str, session: &str, name: &str) -> FileSystem {
        let fs = api::Fuse::mount_with_session(wapm, session, name.to_string());
        let _ = fs.init().join().await;
        FileSystem { fs }
    }

    async fn read_dir(&self, path: &Path) -> FsResult<Dir> {
        debug!("read_dir: path={}", path.display());

        self.fs
            .read_dir(path.to_string_lossy().to_string())
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }

    async fn create_dir(&self, path: &Path) -> FsResult<Metadata> {
        debug!("create_dir: path={}", path.display());

        self.fs
            .create_dir(path.to_string_lossy().to_string())
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }

    async fn remove_dir(&self, path: &Path) -> FsResult<()> {
        debug!("remove_dir: path={}", path.display());

        self.fs
            .remove_dir(path.to_string_lossy().to_string())
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }

    async fn rename(&self, from: &Path, to: &Path) -> FsResult<()> {
        debug!("rename: from={}, to={}", from.display(), to.display());

        self.fs
            .rename(
                from.to_string_lossy().to_string(),
                to.to_string_lossy().to_string(),
            )
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }

    async fn metadata(&self, path: &Path) -> FsResult<Metadata> {
        debug!("metadata: path={}", path.display());

        self.fs
            .read_metadata(path.to_string_lossy().to_string())
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }

    async fn symlink_metadata(&self, path: &Path) -> FsResult<Metadata> {
        debug!("symlink_metadata: path={}", path.display());

        self.fs
            .read_symlink_metadata(path.to_string_lossy().to_string())
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }

    async fn remove_file(&self, path: &Path) -> FsResult<()> {
        debug!("remove_file: path={}", path.display());

        self.fs
            .remove_file(path.to_string_lossy().to_string())
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }

    async fn new_open_options(&self) -> OpenOptions {
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

        let fd = self.fs.fs.open(
            path.to_string_lossy().to_string(),
            api::OpenOptions {
                read: self.conf.read,
                write: self.conf.write,
                create_new: self.conf.create_new,
                create: self.conf.create,
                append: self.conf.append,
                truncate: self.conf.truncate,
            },
        );

        let meta = fd.meta().join().await.map_err(|_| FsError::IOError)??;

        Ok(VirtualFile {
            io: fd.io(),
            fs: self.fs.clone(),
            fd,
            meta,
        })
    }
}

pub struct VirtualFile {
    fs: FileSystem,
    fd: api::OpenedFile,
    io: api::FileIO,
    meta: Metadata,
}

impl Drop for VirtualFile {
    fn drop(&mut self) {
        let _ = self.fd.close();
    }
}

impl io::Seek for VirtualFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            io::SeekFrom::Current(a) => api::SeekFrom::Current(a),
            io::SeekFrom::End(a) => api::SeekFrom::End(a),
            io::SeekFrom::Start(a) => api::SeekFrom::Start(a),
        };

        self.io
            .seek(seek)
            .join()
            .wait()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }
}

impl io::Write for VirtualFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io
            .write(buf.to_vec())
            .join()
            .wait()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
            .map(|a| a as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.io
            .flush()
            .join()
            .wait()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }
}

impl io::Read for VirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data: Result<_, io::Error> = self
            .io
            .read(buf.len() as u64)
            .join()
            .wait()
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
    fn last_accessed(&self) -> u64 {
        self.meta.accessed
    }

    fn last_modified(&self) -> u64 {
        self.meta.modified
    }

    fn created_time(&self) -> u64 {
        self.meta.created
    }

    fn size(&self) -> u64 {
        self.meta.len
    }

    async fn set_len(&mut self, new_size: u64) -> FsResult<()> {
        let result: FsResult<()> = self
            .fd
            .set_len(new_size)
            .join()
            .await
            .map_err(|_| FsError::IOError)?;
        result?;

        self.meta.len = new_size;
        Ok(())
    }

    async fn unlink(&mut self) -> FsResult<()> {
        self.fd
            .unlink()
            .join()
            .await
            .map_err(|_| FsError::IOError)?
    }
}
