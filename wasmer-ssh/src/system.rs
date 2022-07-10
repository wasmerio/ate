use async_trait::async_trait;
use ate::mesh::Registry;
use ate_files::prelude::*;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::io::Read;
use wasmer_os::api::*;
use wasmer_os::err;
use tokio::sync::mpsc;
use wasmer_term::wasmer_os;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use super::NativeFiles;
use std::path::PathBuf;
use super::native_files::NativeFileInterface;
use super::native_files::NativeFileType;

pub struct System {
    pub inner: Arc<dyn SystemAbi>,
    pub native_files: NativeFileInterface,
}

impl System {
    pub async fn new(inner: Arc<dyn SystemAbi>, registry: Arc<Registry>, db_url: url::Url, native_files: NativeFileType) -> Self {
        let native_files = match native_files {
            NativeFileType::AteFileSystem(native_files) => {
                NativeFileInterface::AteFileSystem(NativeFiles::new(registry, db_url, native_files))
            },
            NativeFileType::LocalFileSystem(native_files) => {
                let path = PathBuf::from(native_files);
                NativeFileInterface::LocalFileSystem(path)
            }
        };
        Self {
            inner,
            native_files,
        }
    }
}

#[async_trait]
impl wasmer_os::api::SystemAbi for System {
    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    fn task_shared(
        &self,
        task: Box<
            dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
        >,
    ) {
        self.inner.task_shared(task)
    }

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool that has a stateful thread local variable
    /// It is ok for this task to block execution and any async futures within its scope
    fn task_stateful(
        &self,
        task: Box<
            dyn FnOnce(Rc<RefCell<ThreadLocal>>) -> Pin<Box<dyn Future<Output = ()> + 'static>>
                + Send
                + 'static,
        >,
    ) {
        self.inner.task_stateful(task)
    }

    /// Starts an synchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() + Send + 'static>,
    ) {
        self.inner.task_dedicated(task)
    }

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated_async(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ) {
        self.inner.task_dedicated_async(task)
    }

    /// Starts an asynchronous task on the current thread. This is useful for
    /// launching background work with variables that are not Send.
    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>) {
        self.inner.task_local(task)
    }

    /// Puts the current thread to sleep for a fixed number of milliseconds
    fn sleep(&self, ms: u128) -> AsyncResult<()> {
        self.inner.sleep(ms)
    }

    /// Fetches a data file from the local context of the process
    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, u32>> {
        match &self.native_files {
            NativeFileInterface::AteFileSystem(native_files) => {
                self.fetch_file_via_ate(native_files, path)
            },
            NativeFileInterface::LocalFileSystem(native_files) => {
                self.fetch_file_via_local_fs(native_files, path)
            }
        }
    }

    /// Performs a HTTP or HTTPS request to a destination URL
    fn reqwest(
        &self,
        url: &str,
        method: &str,
        options: ReqwestOptions,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, u32>> {
        self.inner.reqwest(url, method, options, headers, data)
    }

    async fn web_socket(&self, url: &str) -> Result<Box<dyn WebSocketAbi>, String> {
        self.inner.web_socket(url).await
    }

    async fn webgl(&self) -> Option<Box<dyn WebGlAbi>> {
        self.inner.webgl().await
    }
}

fn conv_err(err: FileSystemError) -> u32 {
    match err {
        FileSystemError(FileSystemErrorKind::NoAccess, _) => err::ERR_EACCES,
        FileSystemError(FileSystemErrorKind::PermissionDenied, _) => err::ERR_EPERM,
        FileSystemError(FileSystemErrorKind::ReadOnly, _) => err::ERR_EPERM,
        FileSystemError(FileSystemErrorKind::InvalidArguments, _) => err::ERR_EINVAL,
        FileSystemError(FileSystemErrorKind::NoEntry, _) => err::ERR_ENOENT,
        FileSystemError(FileSystemErrorKind::DoesNotExist, _) => err::ERR_ENOENT,
        FileSystemError(FileSystemErrorKind::AlreadyExists, _) => err::ERR_EEXIST,
        FileSystemError(FileSystemErrorKind::NotDirectory, _) => err::ERR_ENOTDIR,
        FileSystemError(FileSystemErrorKind::IsDirectory, _) => err::ERR_EISDIR,
        FileSystemError(FileSystemErrorKind::NotImplemented, _) => err::ERR_ENOSYS,
        _ => err::ERR_EIO,
    }
}

impl System
{
    fn fetch_file_via_local_fs(&self, native_files: &PathBuf, path: &str) -> AsyncResult<Result<Vec<u8>, u32>> {
        let path = path.to_string();
        let native_files = native_files.clone();
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_dedicated_async(Box::new(move || {
            let task = async move {
                if path.contains("..") || path.contains("~") || path.contains("//") {
                    warn!("relative paths are a security risk - {}", path);
                    return Err(err::ERR_EACCES);
                }
                let mut path = path.as_str();
                while path.starts_with("/") {
                    path = &path[1..];
                }
                let path = native_files.join(path);

                // Attempt to open the file
                let mut file = std::fs::File::open(path.clone())
                    .map_err(|err| {
                        debug!("failed to open local file ({}) - {}", path.to_string_lossy(), err);
                        err::ERR_EIO
                    })?;
                let mut data = Vec::new();
                file
                    .read_to_end(&mut data)
                    .map_err(|err| {
                        debug!("failed to read local file ({}) - {}", path.to_string_lossy(), err);
                        err::ERR_EIO
                    })?;
                Ok(data)
            };
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    fn fetch_file_via_ate(&self, native_files: &NativeFiles, path: &str) -> AsyncResult<Result<Vec<u8>, u32>> {
        let path = path.to_string();
        let native_files = native_files.clone();
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_dedicated_async(Box::new(move || {
            let task = async move {
                let native_files = native_files.get()
                    .await
                    .map_err(|err| {
                        debug!("failed to fetch native files container - {}", err);
                        err::ERR_EIO
                    })?;

                // Search for the file
                let ctx = RequestContext { uid: 0, gid: 0 };
                let flags = ate_files::codes::O_RDONLY as u32;
                let file = native_files
                    .search(&ctx, &path)
                    .await
                    .map_err(conv_err)?
                    .ok_or(err::ERR_ENOENT)?;

                let file = native_files
                    .open(&ctx, file.ino, flags)
                    .await
                    .map_err(conv_err)?;
                let data = native_files
                    .read_all(&ctx, file.inode, file.fh)
                    .await
                    .map_err(conv_err)?;
                Ok(data)
            };
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }
}