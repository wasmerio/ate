#![allow(dead_code)]
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::abi::call;
use crate::abi::Call;
use crate::backend::fuse as backend;

pub use crate::backend::fuse::FsError;

pub type FsResult<T> = Result<T, FsError>;

#[derive(Debug, Clone)]
pub struct FileSystem {
    task: Call,
}

impl FileSystem {
    pub fn mount(wapm: &str, name: &str) -> FileSystem {
        let mount = backend::Mount {
            name: name.to_string(),
        };
        let task = call(wapm.to_string().into(), mount).invoke();
        FileSystem { task }
    }

    async fn read_dir(&self, path: &Path) -> FsResult<backend::Dir> {
        debug!("read_dir: path={}", path.display());

        self.task
            .call(backend::ReadDir {
                path: path.to_string_lossy().to_string(),
            })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
    }

    async fn create_dir(&self, path: &Path) -> FsResult<()> {
        debug!("create_dir: path={}", path.display());

        self.task
            .call(backend::CreateDir {
                path: path.to_string_lossy().to_string(),
            })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
    }

    async fn remove_dir(&self, path: &Path) -> FsResult<()> {
        debug!("remove_dir: path={}", path.display());

        self.task
            .call(backend::RemoveDir {
                path: path.to_string_lossy().to_string(),
            })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
    }

    async fn rename(&self, from: &Path, to: &Path) -> FsResult<()> {
        debug!("rename: from={}, to={}", from.display(), to.display());

        self.task
            .call(backend::Rename {
                from: from.to_string_lossy().to_string(),
                to: to.to_string_lossy().to_string(),
            })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
    }

    async fn metadata(&self, path: &Path) -> FsResult<backend::Metadata> {
        debug!("metadata: path={}", path.display());

        self.task
            .call(backend::ReadMetadata {
                path: path.to_string_lossy().to_string(),
            })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
    }

    async fn symlink_metadata(&self, path: &Path) -> FsResult<backend::Metadata> {
        debug!("symlink_metadata: path={}", path.display());

        self.task
            .call(backend::ReadSymlinkMetadata {
                path: path.to_string_lossy().to_string(),
            })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
    }

    async fn remove_file(&self, path: &Path) -> FsResult<()> {
        debug!("remove_file: path={}", path.display());

        self.task
            .call(backend::RemoveFile {
                path: path.to_string_lossy().to_string(),
            })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
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
        let task = self
            .fs
            .task
            .call(backend::NewOpen {
                read: self.conf.read,
                write: self.conf.write,
                create_new: self.conf.create_new,
                create: self.conf.create,
                append: self.conf.append,
                truncate: self.conf.truncate,
            })
            .invoke();

        let meta = task
            .call(backend::Open {
                path: path.to_string_lossy().to_string(),
            })
            .invoke()
            .join::<FsResult<backend::Metadata>>()
            .await
            .map_err(|_| backend::FsError::IOError)??;

        Ok(VirtualFile {
            fs: self.fs.clone(),
            task,
            meta,
        })
    }
}

pub struct VirtualFile {
    fs: FileSystem,
    task: Call,
    meta: backend::Metadata,
}

impl Seek for VirtualFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            SeekFrom::Current(a) => backend::Seek::Current(a),
            SeekFrom::End(a) => backend::Seek::End(a),
            SeekFrom::Start(a) => backend::Seek::Start(a),
        };

        self.task
            .call(seek)
            .invoke()
            .join::<FsResult<u64>>()
            .wait()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }
}

impl Write for VirtualFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.task
            .call(backend::Write { data: buf.to_vec() })
            .invoke()
            .join::<FsResult<usize>>()
            .wait()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.task
            .call(backend::Flush {})
            .invoke()
            .join::<FsResult<()>>()
            .wait()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into())
    }
}

impl Read for VirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data: Result<Vec<u8>, io::Error> = self
            .task
            .call(backend::Flush {})
            .invoke()
            .join::<FsResult<Vec<u8>>>()
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
            .task
            .call(backend::SetLength { len: new_size })
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?;
        result?;

        self.meta.len = new_size;
        Ok(())
    }

    async fn unlink(&mut self) -> FsResult<()> {
        self.task
            .call(backend::Unlink {})
            .invoke()
            .join()
            .await
            .map_err(|_| backend::FsError::IOError)?
    }
}
