use async_trait::async_trait;
use wasmer_term::wasmer_os::wasmer::Module;
use wasmer_term::wasmer_os::wasmer::Store;
use wasmer_term::wasmer_os::wasmer::vm::VMMemory;
use wasmer_term::wasmer_os::wasmer_wasi::WasiThreadError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::io::Read;
use wasmer_os::api::*;
use wasmer_os::err;
use tokio::sync::mpsc;
use wasmer_term::wasmer_os;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use std::path::PathBuf;
use super::native_files::NativeFileType;

pub struct System {
    pub inner: Arc<dyn SystemAbi>,
    pub native_files: NativeFileType,
}

impl System {
    pub async fn new(inner: Arc<dyn SystemAbi>, native_files: NativeFileType) -> Self {
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
    fn task_wasm(
        &self,
        task: Box<dyn FnOnce(Store, Module, Option<VMMemory>) -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
        store: Store,
        module: Module,
        spawn_type: SpawnType,
    ) -> Result<(), WasiThreadError> {
        self.inner.task_wasm(task, store, module, spawn_type)
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
            NativeFileType::LocalFileSystem(native_files) => {
                let native_files = PathBuf::from(native_files);
                self.fetch_file_via_local_fs(&native_files, path)
            },
            NativeFileType::EmbeddedFiles => {
                self.inner.fetch_file(path)
            },
            NativeFileType::None => {
                AsyncResult::new_static(SerializationFormat::Bincode, Err(err::ERR_ENOENT))
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
}