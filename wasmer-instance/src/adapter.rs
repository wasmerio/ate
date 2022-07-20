use std::sync::Arc;
use std::path::Path;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::sync::Mutex;
use std::future::Future;
use std::task::Context;
use std::task::Poll;
use ate_files::accessor::FileAccessor;
use ate_files::accessor::RequestContext;
use wasmer_ssh::wasmer_os;
use wasmer_os::fs::MountedFileSystem;
use wasmer_os::bus::WasmCallerContext;
use wasmer_os::wasmer_vfs::*;
use wasmer_bus_fuse::api::FileSystemSimplified;
use wasmer_bus_fuse::api::OpenedFileSimplified;
use wasmer_bus_fuse::api::FileIOSimplified;
use wasmer_bus_fuse::api as backend;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Clone)]
pub struct FileAccessorAdapter
{
    ctx: Arc<Mutex<Option<WasmCallerContext>>>,
    inner: wasmer_deploy_cli::bus::file_system::FileSystem,
}

impl FileAccessorAdapter
{
    pub fn new(accessor: &Arc<FileAccessor>) -> Self {
        Self {
            ctx: Arc::new(Mutex::new(None)),
            inner: wasmer_deploy_cli::bus::file_system::FileSystem::new(
                accessor.clone(),
                RequestContext {
                    uid: 0,
                    gid: 0,
                }
            )
        }
    }

    pub fn get_ctx(&self) -> WasmCallerContext
    {
        let guard = self.ctx.lock().unwrap();
        guard.clone().unwrap_or_default()
    }

    fn block_on<Fut>(&self, fut: Fut) -> Result<Fut::Output>
    where Fut: Future,
          Fut: Send + 'static
    {
        let mut fut = Box::pin(fut);
        tokio::task::block_in_place(move || {
            let mut timer = None;
            let waker = dummy_waker::dummy_waker();
            let mut cx = Context::from_waker(&waker);
            let mut tick_wait = 0u64;
            loop {
                // Attempt to process it
                let fut = fut.as_mut();
                if let Poll::Ready(a) = fut.poll(&mut cx) {
                    return Ok(a)
                }

                // Set the timer if there is none
                if timer.is_none() {
                    timer.replace(std::time::Instant::now());
                }
                let timer = timer.as_ref().unwrap();

                // Check the context to see if we need to terminate
                {
                    let guard = self.ctx.lock().unwrap();
                    if let Some(ctx) = guard.as_ref() {
                        if ctx.should_terminate().is_some() {
                            return Err(FsError::Interrupted);
                        }
                    }
                }

                // If too much time has elapsed then fail
                let elapsed = timer.elapsed();
                if elapsed > std::time::Duration::from_secs(30) {
                    return Err(FsError::TimedOut);
                }

                // Linearly increasing wait time
                tick_wait += 1;
                let wait_time = u64::min(tick_wait / 10, 20);
                std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
            }
        })
    }
}

impl MountedFileSystem
for FileAccessorAdapter
{
    fn set_ctx(&self, ctx: &WasmCallerContext) {
        let mut guard = self.ctx.lock().unwrap();
        guard.replace(ctx.clone());
    }
}

impl FileSystem
for FileAccessorAdapter
{
    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        let path = path.to_string_lossy().to_string();
        let inner = self.inner.clone();
        
        self.block_on(async move {
            inner.read_dir(path).await
        })?
        .map_err(conv_fs_err)
        .map(conv_dir)
    }
    
    fn create_dir(&self, path: &Path) -> Result<()> {
        let path = path.to_string_lossy().to_string();
        let inner = self.inner.clone();
        self.block_on(async move {
            inner.create_dir(path).await
        })?
        .map_err(conv_fs_err)?;
        Ok(())
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        let path = path.to_string_lossy().to_string();
        let inner = self.inner.clone();
        self.block_on(async move {
            inner.remove_dir(path).await
        })?
        .map_err(conv_fs_err)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let from = from.to_string_lossy().to_string();
        let to = to.to_string_lossy().to_string();
        let inner = self.inner.clone();
        self.block_on(async move {
            inner.rename(from, to).await
        })?
        .map_err(conv_fs_err)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata> {
        let path = path.to_string_lossy().to_string();
        let inner = self.inner.clone();
        self.block_on(async move {
            inner.read_metadata(path).await
        })?
        .map_err(conv_fs_err)
        .map(conv_metadata)
    }

    fn symlink_metadata(&self, path: &Path) -> Result<Metadata> {
        let path = path.to_string_lossy().to_string();
        let inner = self.inner.clone();
        self.block_on(async move {
            inner.read_symlink_metadata(path).await
        })?
        .map_err(conv_fs_err)
        .map(conv_metadata)
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        let path = path.to_string_lossy().to_string();
        let inner = self.inner.clone();
        self.block_on(async move {
            inner.remove_file(path).await
        })?
        .map_err(conv_fs_err)
    }

    fn new_open_options(&self) -> OpenOptions {
        return OpenOptions::new(Box::new(FileAccessorOpener::new(self)));
    }
}

#[derive(Debug)]
pub struct FileAccessorOpener {
    fs: FileAccessorAdapter,
}

impl FileAccessorOpener {
    pub fn new(fs: &FileAccessorAdapter) -> Self {
        Self { fs: fs.clone() }
    }
}

impl FileOpener for FileAccessorOpener {
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> Result<Box<dyn VirtualFile + Send + Sync>> {
        debug!("open: path={}", path.display());

        let path = path.to_string_lossy().to_string();
        let options = backend::OpenOptions {
            read: conf.read(),
            write: conf.write(),
            create_new: conf.create_new(),
            create: conf.create(),
            append: conf.append(),
            truncate: conf.truncate(),
        };

        let inner = self.fs.inner.clone();
        let (file, io) = self.fs.block_on(async move {
            let file = inner.open(path, options).await;
            let io = file.io().await;
            (file, io)
        })?;
        let io = io
        .map_err(|_| FsError::BrokenPipe)?;

        let of = file.clone();
        let meta = self.fs.block_on(async move {
            of.meta().await
        })?
        .map_err(conv_fs_err)?;

        return Ok(Box::new(FileAccessorVirtualFile {
            fs: self.fs.clone(),
            of: file,
            io,
            meta,
            dirty: conf.create_new() || conf.truncate(),
        }));
    }
}

#[derive(Debug)]
pub struct FileAccessorVirtualFile {
    fs: FileAccessorAdapter,
    of: Arc<wasmer_deploy_cli::bus::opened_file::OpenedFile>,
    io: Arc<wasmer_deploy_cli::bus::file_io::FileIo>,
    meta: backend::Metadata,
    dirty: bool,
}

impl Drop
for FileAccessorVirtualFile
{
    fn drop(&mut self) {
        if self.dirty {
            let _ = self.flush();
        }
    }
}

impl Seek for FileAccessorVirtualFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            SeekFrom::Current(a) => backend::SeekFrom::Current(a),
            SeekFrom::End(a) => backend::SeekFrom::End(a),
            SeekFrom::Start(a) => backend::SeekFrom::Start(a),
        };

        let io = self.io.clone();
        self.fs.block_on(async move {
            io.seek(seek).await
        })
        .map_err(|_| conv_io_err(backend::FsError::ConnectionAborted))?
        .map_err(conv_io_err)
    }
}

impl Write for FileAccessorVirtualFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf = buf.to_vec();
        let io = self.io.clone();
        
        let ret = self.fs.block_on(async move {
            io.write(buf).await
        })
        .map_err(|_| conv_io_err(backend::FsError::ConnectionAborted))?
        .map_err(conv_io_err)
        .map(|a| a as usize)?;

        self.dirty = true;
        Ok(ret)
    }

    fn flush(&mut self) -> io::Result<()> {
        let io = self.io.clone();
        self.fs.block_on(async move {
            io.flush().await
        })
        .map_err(|_| conv_io_err(backend::FsError::ConnectionAborted))?
        .map_err(conv_io_err)?;

        self.dirty = false;
        Ok(())
    }
}

impl Read for FileAccessorVirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len();
        let io = self.io.clone();
        
        let data = self.fs.block_on(async move {
            io.read(len as u64).await
        })
        .map_err(|_| conv_io_err(backend::FsError::ConnectionAborted))?
        .map_err(conv_io_err)?;

        if data.len() <= 0 {
            return Ok(0usize);
        }

        let dst = &mut buf[..data.len()];
        dst.copy_from_slice(&data[..]);
        Ok(data.len())
    }
}

impl VirtualFile for FileAccessorVirtualFile {
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

    fn set_len(&mut self, new_size: u64) -> Result<()> {
        let of = self.of.clone();
        self.fs.block_on(async move {
            of.set_len(new_size).await
        })?
        .map_err(conv_fs_err)?;

        self.dirty = true;
        self.meta.len = new_size;
        Ok(())
    }

    fn unlink(&mut self) -> Result<()> {
        let of = self.of.clone();
        self.fs.block_on(async move {
            of.unlink().await
        })?
        .map_err(conv_fs_err)?;
        
        self.dirty = false;
        Ok(())
    }
}

fn conv_dir(dir: backend::Dir) -> ReadDir {
    ReadDir::new(
        dir.data
            .into_iter()
            .map(|a| conv_dir_entry(a))
            .collect::<Vec<_>>(),
    )
}

fn conv_dir_entry(entry: backend::DirEntry) -> DirEntry {
    DirEntry {
        path: Path::new(entry.path.as_str()).to_owned(),
        metadata: entry
            .metadata
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

fn conv_io_err(err: wasmer_bus_fuse::api::FsError) -> io::Error {
    err.into()
}

fn conv_fs_err(err: wasmer_bus_fuse::api::FsError) -> FsError {
    use wasmer_bus_fuse::api::FsError as E;
    match err {
        E::BaseNotDirectory => FsError::BaseNotDirectory,
        E::NotAFile => FsError::NotAFile,
        E::InvalidFd => FsError::InvalidFd,
        E::AlreadyExists => FsError::AlreadyExists,
        E::Lock => FsError::Lock,
        E::IOError => FsError::IOError,
        E::AddressInUse => FsError::AddressInUse,
        E::AddressNotAvailable => FsError::AddressNotAvailable,
        E::BrokenPipe => FsError::BrokenPipe,
        E::ConnectionAborted => FsError::ConnectionAborted,
        E::ConnectionRefused => FsError::ConnectionRefused,
        E::ConnectionReset => FsError::ConnectionReset,
        E::Interrupted => FsError::Interrupted,
        E::InvalidData => FsError::InvalidData,
        E::InvalidInput => FsError::InvalidInput,
        E::NotConnected => FsError::NotConnected,
        E::EntityNotFound => FsError::EntityNotFound,
        E::NoDevice => FsError::NoDevice,
        E::PermissionDenied => FsError::PermissionDenied,
        E::TimedOut => FsError::TimedOut,
        E::UnexpectedEof => FsError::UnexpectedEof,
        E::WouldBlock => FsError::WouldBlock,
        E::WriteZero => FsError::WriteZero,
        E::DirectoryNotEmpty => FsError::DirectoryNotEmpty,
        E::UnknownError => FsError::UnknownError,
    }
}