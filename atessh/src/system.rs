use async_trait::async_trait;
use ate_files::prelude::*;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use term_lib::api::*;
use term_lib::err;
use tokio::sync::mpsc;
use tokterm::term_lib;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

pub struct System {
    pub inner: Arc<dyn SystemAbi>,
    pub native_files: Arc<FileAccessor>,
}

impl System {
    pub fn new(inner: Arc<dyn SystemAbi>, native_files: Arc<FileAccessor>) -> Self {
        Self {
            inner,
            native_files,
        }
    }
}

#[async_trait]
impl term_lib::api::SystemAbi for System {
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

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ) {
        self.inner.task_dedicated(task)
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
        let path = path.to_string();
        let native_files = self.native_files.clone();
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_dedicated(Box::new(move || {
            let task = async move {
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

    /// Performs a HTTP or HTTPS request to a destination URL
    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, u32>> {
        self.inner.reqwest(url, method, headers, data)
    }

    async fn web_socket(&self, url: &str) -> Result<Box<dyn WebSocketAbi>, String> {
        self.inner.web_socket(url).await
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
