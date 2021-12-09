#![allow(unused_variables, dead_code)]
use derivative::*;
use std::path::Path;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_vfs::FileOpener;
use wasmer_vfs::FileSystem;
use wasmer_vfs::FsError;
use wasmer_vfs::Metadata;
use wasmer_vfs::OpenOptions;
use wasmer_vfs::OpenOptionsConfig;
use wasmer_vfs::ReadDir;
use wasmer_vfs::DirEntry;
use wasmer_vfs::VirtualFile;
use wasmer_vfs::FileType;
use wasm_bus::backend::fuse as backend;
use std::io;
use std::io::Read;
use std::io::Write;
use std::io::Seek;
use std::io::SeekFrom;

use crate::api::*;
use crate::bus::SubProcess;
use crate::bus::AsyncWasmBusResult;

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct FuseFileSystem {
    system: System,
    #[derivative(Debug = "ignore")]
    sub: SubProcess,
    target: String,
}

impl FuseFileSystem {
    pub fn new(process: SubProcess, target: &str) -> FuseFileSystem {
        FuseFileSystem {
            system: System::default(),
            sub: process,
            target: target.to_string(),
        }
    }
}

impl FileSystem for FuseFileSystem {
    fn read_dir(&self, path: &Path) -> Result<ReadDir, FsError> {
        debug!("read_dir: path={}", path.display());

        let dir = self.sub.main
            .call(backend::ReadDir {
                path: path.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?;

        Ok(conv_dir(dir))
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        debug!("create_dir: path={}", path.display());
        
        self.sub.main
            .call(backend::CreateDir {
                path: path.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        debug!("remove_dir: path={}", path.display());
        
        self.sub.main
            .call(backend::RemoveDir {
                path: path.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        debug!("rename: from={}, to={}", from.display(), to.display());
        
        self.sub.main
            .call(backend::Rename {
                from: from.to_string_lossy().to_string(),
                to: to.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        debug!("metadata: path={}", path.display());
        
        let metadata = self.sub.main
            .call(backend::ReadMetadata {
                path: path.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?;

        Ok(conv_metadata(metadata))
    }

    fn symlink_metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        debug!("symlink_metadata: path={}", path.display());

        let metadata = self.sub.main
            .call(backend::ReadSymlinkMetadata {
                path: path.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?;

        Ok(conv_metadata(metadata))
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        debug!("remove_file: path={}", path.display());
        
        self.sub.main
            .call(backend::RemoveFile {
                path: path.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)
    }

    fn new_open_options(&self) -> OpenOptions {
        return OpenOptions::new(Box::new(FuseFileOpener::new(self)));
    }
}

#[derive(Debug)]
pub struct FuseFileOpener {
    fs: FuseFileSystem,
}

impl FuseFileOpener {
    pub fn new(fs: &FuseFileSystem) -> FuseFileOpener {
        FuseFileOpener {
            fs: fs.clone(),
        }
    }
}

impl FileOpener for FuseFileOpener {
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> Result<Box<dyn VirtualFile>, FsError> {
        debug!("open: path={}", path.display());
        
        let task: AsyncWasmBusResult<()> = self.fs.sub.main
            .call(backend::NewOpen {
                read: conf.read(),
                write: conf.write(),
                create_new: conf.create_new(),
                create: conf.create(),
                append: conf.append(),
                truncate: conf.truncate(),
            })
            .map_err(|_| FsError::IOError)?;

        let meta = task
            .call(backend::Open {
                path: path.to_string_lossy().to_string(),
            })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?;

        return Ok(Box::new(FuseVirtualFile {
            fs: self.fs.clone(),
            task,
            meta,
        }));
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FuseVirtualFile {
    fs: FuseFileSystem,
    #[derivative(Debug = "ignore")]
    task: AsyncWasmBusResult<()>,
    meta: backend::Metadata,
}

impl Seek for FuseVirtualFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            SeekFrom::Current(a) => backend::Seek::Current(a),
            SeekFrom::End(a) => backend::Seek::End(a),
            SeekFrom::Start(a) => backend::Seek::Start(a),
        };

        let ret: io::Result<_> = self.task
            .call::<Result<_, backend::FsError>, _>(seek)
            .map_err(|err| err.into_io_error())?
            .block_on()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());
        Ok(ret?)
    }
}

impl Write for FuseVirtualFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let ret: io::Result<_> = self.task
            .call::<Result<_, backend::FsError>, _>(backend::Write { data: buf.to_vec() })
            .map_err(|err| err.into_io_error())?
            .block_on()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());
        Ok(ret?)
    }

    fn flush(&mut self) -> io::Result<()> {
        let ret: io::Result<_> = self.task
            .call::<Result<_, backend::FsError>, _>(backend::Flush {})
            .map_err(|err| err.into_io_error())?
            .block_on()
            .map_err(|err| err.into_io_error())?
            .map_err(|err| err.into());
        Ok(ret?)
    }
}

impl Read for FuseVirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data: io::Result<Vec<u8>> = self
            .task
            .call::<Result<_, backend::FsError>, _>(backend::Flush {})
            .map_err(|err| err.into_io_error())?
            .block_on()
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

impl VirtualFile
for FuseVirtualFile {
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

    fn set_len(&mut self, new_size: u64) -> Result<(), FsError> {
        let result: Result<(), FsError> = self.task
            .call::<Result<_, backend::FsError>, _>(backend::SetLength { len: new_size })
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(|err| conv_fs_error(err));
        result?;
        
        self.meta.len = new_size;
        Ok(())
    }

    fn unlink(&mut self) -> Result<(), FsError> {
        self.task
            .call::<Result<_, backend::FsError>, _>(backend::Unlink {})
            .map_err(|_| FsError::IOError)?
            .block_on()
            .map_err(|_| FsError::IOError)?
            .map_err(|err| conv_fs_error(err))
    }
}

fn conv_dir(dir: backend::Dir) -> ReadDir {
    ReadDir::new(dir.data
            .into_iter()
            .map(|a| conv_dir_entry(a))
            .collect::<Vec<_>>()
    )
}

fn conv_dir_entry(entry: backend::DirEntry) -> DirEntry {
    DirEntry {
        path: Path::new(entry.path.as_str()).to_owned(),
        metadata: entry.metadata
            .ok_or_else(|| FsError::IOError)
            .map(|a| conv_metadata(a)),
    }
}

fn conv_metadata(metadata: backend::Metadata) -> Metadata {
    Metadata {
        ft: conv_file_type(metadata.ft),
        accessed: metadata.accessed,
        created: metadata.created,
        modified: metadata.modified,
        len: metadata.len,
    }
}

fn conv_file_type(ft: backend::FileType) -> FileType {
    FileType {
        dir: ft.dir,
        file: ft.file,
        symlink: ft.symlink,
        char_device: ft.char_device,
        block_device: ft.block_device,
        socket: ft.socket,
        fifo: ft.fifo,
    }
}

fn conv_fs_error(err: backend::FsError) -> FsError {
    match err {
        backend::FsError::BaseNotDirectory => FsError::BaseNotDirectory,
        backend::FsError::NotAFile => FsError::NotAFile,
        backend::FsError::InvalidFd => FsError::InvalidFd,
        backend::FsError::AlreadyExists => FsError::AlreadyExists,
        backend::FsError::Lock => FsError::Lock,
        backend::FsError::IOError => FsError::IOError,
        backend::FsError::AddressInUse => FsError::AddressInUse,
        backend::FsError::AddressNotAvailable => FsError::AddressNotAvailable,
        backend::FsError::BrokenPipe => FsError::BrokenPipe,
        backend::FsError::ConnectionAborted => FsError::ConnectionAborted,
        backend::FsError::ConnectionRefused => FsError::ConnectionRefused,
        backend::FsError::ConnectionReset => FsError::ConnectionReset,
        backend::FsError::Interrupted => FsError::Interrupted,
        backend::FsError::InvalidData => FsError::InvalidData,
        backend::FsError::InvalidInput => FsError::InvalidInput,
        backend::FsError::NotConnected => FsError::NotConnected,
        backend::FsError::EntityNotFound => FsError::EntityNotFound,
        backend::FsError::NoDevice => FsError::NoDevice,
        backend::FsError::PermissionDenied => FsError::PermissionDenied,
        backend::FsError::TimedOut => FsError::TimedOut,
        backend::FsError::UnexpectedEof => FsError::UnexpectedEof,
        backend::FsError::WouldBlock => FsError::WouldBlock,
        backend::FsError::WriteZero => FsError::WriteZero,
        backend::FsError::DirectoryNotEmpty => FsError::DirectoryNotEmpty,
        backend::FsError::UnknownError => FsError::UnknownError,
    }
}