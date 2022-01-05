#![allow(dead_code)]
#![allow(unused)]
use std::io::prelude::*;
use std::io::SeekFrom;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api::SystemAbiExt;
use crate::wasmer_vfs::Result as FsResult;
use crate::wasmer_vfs::*;
use crate::wasmer_vfs::{FileDescriptor, VirtualFile};
use crate::wasmer_wasi::{types as wasi_types, WasiFile, WasiFsError};

use crate::fd::*;
use crate::stdio::*;
use crate::tty::*;

#[derive(Debug)]
pub struct ProcFileSystem {
    type_file: FileType,
    type_dir: FileType,
    type_char: FileType,
    stdio: Stdio,
}

impl ProcFileSystem {
    pub fn new(stdio: Stdio) -> ProcFileSystem {
        let mut ret = ProcFileSystem {
            type_file: FileType::default(),
            type_dir: FileType::default(),
            type_char: FileType::default(),
            stdio,
        };
        ret.type_file.file = true;
        ret.type_dir.dir = true;
        ret.type_char.char_device = true;

        ret
    }
}

impl ProcFileSystem {
    fn default_metadata(type_: &FileType) -> Metadata {
        Metadata {
            ft: type_.clone(),
            accessed: 0,
            created: 0,
            modified: 0,
            len: 0,
        }
    }

    fn default_metadata_with_size(type_: &FileType, size: usize) -> Metadata {
        Metadata {
            ft: type_.clone(),
            accessed: 0,
            created: 0,
            modified: 0,
            len: size as u64,
        }
    }
}

impl FileSystem for ProcFileSystem {
    fn read_dir(&self, path: &Path) -> FsResult<ReadDir> {
        debug!("read_dir: path={}", path.display());

        let mut entries = Vec::new();
        let path = path.to_string_lossy();
        let path = path.as_ref();
        match path {
            "/" | "" => {
                entries.push(DirEntry {
                    path: PathBuf::from("stdin"),
                    metadata: Ok(Self::default_metadata(&self.type_file)),
                });
                entries.push(DirEntry {
                    path: PathBuf::from("stdout"),
                    metadata: Ok(Self::default_metadata(&self.type_file)),
                });
                entries.push(DirEntry {
                    path: PathBuf::from("stderr"),
                    metadata: Ok(Self::default_metadata(&self.type_file)),
                });
                entries.push(DirEntry {
                    path: PathBuf::from("log"),
                    metadata: Ok(Self::default_metadata(&self.type_file)),
                });
                entries.push(DirEntry {
                    path: PathBuf::from("null"),
                    metadata: Ok(Self::default_metadata(&self.type_file)),
                });
                entries.push(DirEntry {
                    path: PathBuf::from("tty"),
                    metadata: Ok(Self::default_metadata(&self.type_file)),
                });
            }
            _ => {
                return Err(FsError::EntityNotFound);
            }
        }
        Ok(ReadDir::new(entries))
    }
    fn create_dir(&self, path: &Path) -> FsResult<()> {
        debug!("create_dir: path={}", path.display());
        Err(FsError::EntityNotFound)
    }
    fn remove_dir(&self, path: &Path) -> FsResult<()> {
        debug!("remove_dir: path={}", path.display());
        Err(FsError::EntityNotFound)
    }
    fn rename(&self, from: &Path, to: &Path) -> FsResult<()> {
        debug!("rename: from={} to={}", from.display(), to.display());
        Err(FsError::EntityNotFound)
    }
    fn metadata(&self, path: &Path) -> FsResult<Metadata> {
        debug!("metadata: path={}", path.display());
        let path = path.to_string_lossy();
        let path = path.as_ref();
        match path {
            "/" | "" => Ok(Self::default_metadata(&self.type_dir)),
            "/stdin" | "stdin" => Ok(Self::default_metadata(&self.type_file)),
            "/stdout" | "stdout" => Ok(Self::default_metadata(&self.type_file)),
            "/stderr" | "stderr" => Ok(Self::default_metadata(&self.type_file)),
            "/log" | "log" => Ok(Self::default_metadata(&self.type_file)),
            "/null" | "null" => Ok(Self::default_metadata(&self.type_file)),
            "/tty" | "tty" => Ok(Self::default_metadata(&self.type_file)),
            _ => Err(FsError::EntityNotFound),
        }
    }
    fn symlink_metadata(&self, path: &Path) -> FsResult<Metadata> {
        debug!("symlink_metadata: path={}", path.display());
        self.metadata(path)
    }
    fn remove_file(&self, path: &Path) -> FsResult<()> {
        debug!("remove_file: path={}", path.display());
        Err(FsError::EntityNotFound)
    }
    fn new_open_options(&self) -> OpenOptions {
        let opener = Box::new(CoreFileOpener {
            stdio: self.stdio.clone(),
        });
        OpenOptions::new(opener)
    }
}

#[derive(Debug)]
pub struct CoreFileOpener {
    stdio: Stdio,
}

impl FileOpener for CoreFileOpener {
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> FsResult<Box<dyn VirtualFile + Sync>> {
        debug!("open: path={}", path.display());
        let path = path.to_string_lossy();
        let path = path.as_ref();
        match path {
            "/stdin" | "stdin" => Ok(Box::new(self.stdio.stdin.clone())),
            "/stdout" | "stdout" => Ok(Box::new(self.stdio.stdout.clone())),
            "/stderr" | "stderr" => Ok(Box::new(self.stdio.stderr.clone())),
            "/log" | "log" => Ok(Box::new(self.stdio.log.clone())),
            "/null" | "null" => Ok(Box::new(NullFile::default())),
            "/tty" | "tty" => Ok(Box::new(TtyFile::new(&self.stdio))),
            _ => Err(FsError::EntityNotFound),
        }
    }
}

#[derive(Debug)]
pub struct TtyFile {
    fd: Fd,
    tty: Tty,
}

impl TtyFile {
    pub fn new(stdio: &Stdio) -> TtyFile {
        let mut fd = Fd::combine(&stdio.stdin, &stdio.stdout);
        fd.set_flag(FdFlag::Tty);

        stdio.tty.set_buffering(false);
        TtyFile {
            fd,
            tty: stdio.tty.clone(),
        }
    }
}

impl Drop for TtyFile {
    fn drop(&mut self) {
        self.tty.set_buffering(true);
    }
}

impl Seek for TtyFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.fd.seek(pos)
    }
}

impl Write for TtyFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.fd.write_all(buf)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.fd.flush()
    }
}

impl Read for TtyFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fd.read(buf)
    }
}

impl VirtualFile for TtyFile {
    fn last_accessed(&self) -> u64 {
        self.fd.last_accessed()
    }
    fn last_modified(&self) -> u64 {
        self.fd.last_modified()
    }
    fn created_time(&self) -> u64 {
        self.fd.created_time()
    }
    fn size(&self) -> u64 {
        self.fd.size()
    }
    fn set_len(&mut self, new_size: wasi_types::__wasi_filesize_t) -> StdResult<(), WasiFsError> {
        self.fd.set_len(new_size)
    }
    fn unlink(&mut self) -> StdResult<(), WasiFsError> {
        self.fd.unlink()
    }
    fn bytes_available(&self) -> StdResult<usize, WasiFsError> {
        self.fd.bytes_available()
    }
    fn get_fd(&self) -> Option<FileDescriptor> {
        self.fd.get_fd()
    }
}

#[derive(Debug, Default)]
pub struct NullFile {}

impl Seek for NullFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}
impl Write for NullFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for NullFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl VirtualFile for NullFile {
    fn last_accessed(&self) -> u64 {
        0
    }
    fn last_modified(&self) -> u64 {
        0
    }
    fn created_time(&self) -> u64 {
        0
    }
    fn size(&self) -> u64 {
        0
    }
    fn set_len(&mut self, new_size: wasi_types::__wasi_filesize_t) -> StdResult<(), WasiFsError> {
        Ok(())
    }
    fn unlink(&mut self) -> StdResult<(), WasiFsError> {
        Ok(())
    }
    fn bytes_available(&self) -> StdResult<usize, WasiFsError> {
        Ok(0)
    }
    fn get_fd(&self) -> Option<FileDescriptor> {
        None
    }
}

#[derive(Debug)]
pub struct LogFile {
    prefix: String,
    fd: Fd,
}

impl LogFile {
    pub fn new(prefix: String, stdio: &Stdio) -> LogFile {
        LogFile {
            prefix,
            fd: stdio.log.clone(),
        }
    }
}

impl Seek for LogFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}

impl Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let ret = buf.len();
        let prefix = self.prefix.as_bytes();

        let mut to_write = Vec::with_capacity(prefix.len() + buf.len());
        to_write.write_all(prefix)?;
        to_write.write_all(buf)?;

        self.fd.write_all(&to_write[..])?;
        Ok(ret)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.fd.flush()
    }
}

impl Read for LogFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl VirtualFile for LogFile {
    fn last_accessed(&self) -> u64 {
        0
    }
    fn last_modified(&self) -> u64 {
        0
    }
    fn created_time(&self) -> u64 {
        0
    }
    fn size(&self) -> u64 {
        0
    }
    fn set_len(&mut self, new_size: wasi_types::__wasi_filesize_t) -> StdResult<(), WasiFsError> {
        Ok(())
    }
    fn unlink(&mut self) -> StdResult<(), WasiFsError> {
        Ok(())
    }
    fn bytes_available(&self) -> StdResult<usize, WasiFsError> {
        Ok(0)
    }
    fn get_fd(&self) -> Option<FileDescriptor> {
        None
    }
}
