use async_trait::async_trait;
use tracing::error;
#[cfg(feature = "sys")]
use wasmer::MemoryStyle;
use wasmer::MemoryType;
use wasmer::VMMemory;
use wasmer_wasi::WasiThreadError;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;
use wasmer_bus::abi::SerializationFormat;

use super::*;

#[derive(Debug, Clone)]
pub struct ConsoleRect {
    pub cols: u32,
    pub rows: u32,
}

pub struct ReqwestOptions {
    pub gzip: bool,
    pub cors_proxy: Option<String>,
}

pub struct ReqwestResponse {
    pub pos: usize,
    pub data: Option<Vec<u8>>,
    pub ok: bool,
    pub redirected: bool,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
}

// This ABI implements a set of emulated operating system
// functions that are specific to a console session
#[async_trait]
pub trait ConsoleAbi
where
    Self: Send + Sync,
{
    /// Writes output to the console
    async fn stdout(&self, data: Vec<u8>);

    /// Writes output to the console
    async fn stderr(&self, data: Vec<u8>);

    /// Flushes the output to the console
    async fn flush(&self);

    /// Writes output to the log
    async fn log(&self, text: String);

    /// Gets the number of columns and rows in the terminal
    async fn console_rect(&self) -> ConsoleRect;

    /// Clears the terminal
    async fn cls(&self);

    /// Tell the process to exit (if it can)
    async fn exit(&self);
}

// This ABI implements a number of low level operating system
// functions that this terminal depends upon
#[async_trait]
pub trait SystemAbi
where
    Self: Send + Sync,
{
    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    fn task_shared(
        &self,
        task: Box<
            dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
        >,
    );

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool that has a stateful thread local variable
    /// It is ok for this task to block execution and any async futures within its scope
    fn task_wasm(
        &self,
        task: Box<dyn FnOnce(Option<VMMemory>) -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
        spawn_type: SpawnType,
    ) -> Result<(), WasiThreadError>;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() + Send + 'static>,
    );

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated_async(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    );

    /// Starts an asynchronous task on the current thread. This is useful for
    /// launching background work with variables that are not Send.
    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>);

    /// Puts the current thread to sleep for a fixed number of milliseconds
    fn sleep(&self, ms: u128) -> AsyncResult<()>;

    /// Fetches a data file from the local context of the process
    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, u32>>;

    /// Performs a HTTP or HTTPS request to a destination URL
    fn reqwest(
        &self,
        url: &str,
        method: &str,
        options: ReqwestOptions,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, u32>>;

    /// Make a web socket connection to a particular URL
    async fn web_socket(&self, url: &str) -> Result<Box<dyn WebSocketAbi>, String>;

    /// Open the WebGL
    async fn webgl(&self) -> Option<Box<dyn WebGlAbi>>;
}

#[derive(Debug)]
pub enum SpawnType {
    Create,
    #[cfg(feature = "sys")]
    CreateWithTypeAndStyle(MemoryType, MemoryStyle),
    #[cfg(feature = "js")]
    CreateWithType(MemoryType),
    NewThread(VMMemory),
}

// System call extensions that provide generics
#[async_trait]
pub trait SystemAbiExt {
    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    fn spawn_shared<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send;

    /// Starts an web assembly task will will run on a dedicated thread
    /// It is ok for this task to block execution and any async futures within its scope
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    #[cfg(feature = "sys")]
    fn spawn_wasm<F, Fut>(&self, task: F, memory: SpawnType) -> AsyncResult<Fut::Output>
    where
        F: FnOnce(Option<VMMemory>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send;

    /// Starts an web assembly task will will run on a dedicated thread
    /// It is ok for this task to block execution and any async futures within its scope
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    #[cfg(feature = "js")]
    fn spawn_wasm<F, Fut>(&self, task: F, memory: SpawnType) -> AsyncResult<Fut::Output>
    where
        F: FnOnce(Option<VMMemory>) -> Fut,
        F: 'static,
        Fut: Future + 'static,
        Fut::Output: Send;

    /// Starts an synchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    fn spawn_dedicated<F>(&self, task: F)
    where
        F: FnOnce(),
        F: Send + 'static;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    fn spawn_dedicated_async<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send;

    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    /// This is the fire-and-forget variet of spawning background work
    fn fork_shared<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send;

    /// Attempts to send the message instantly however if that does not
    /// work it spawns a background thread and sends it there instead
    fn fire_and_forget<T: Send + 'static>(&self, sender: &mpsc::Sender<T>, msg: T);

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool that has a stateful thread local variable
    /// It is ok for this task to block execution and any async futures within its scope
    /// This is the fire-and-forget variet of spawning background work
    fn fork_wasm<F, Fut>(&self, task: F, spawn_type: SpawnType)
    where
        F: FnOnce(Option<VMMemory>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// This is the fire-and-forget variet of spawning background work
    fn fork_dedicated<F>(&self, task: F)
    where
        F: FnOnce(),
        F: Send + 'static;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// This is the fire-and-forget variet of spawning background work
    fn fork_dedicated_async<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static;

    /// Starts an asynchronous task on the current thread. This is useful for
    /// launching background work with variables that are not Send.
    /// This is the fire-and-forget variet of spawning background work
    fn fork_local<F>(&self, task: F)
    where
        F: Future + 'static;
}

#[async_trait]
impl SystemAbiExt for dyn SystemAbi {
    fn spawn_shared<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_shared(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    #[cfg(feature = "sys")]
    fn spawn_wasm<F, Fut>(&self, task: F, spawn_type: SpawnType) -> AsyncResult<Fut::Output>
    where
        F: FnOnce(Option<VMMemory>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        if let Err(err) = self.task_wasm(Box::new(move |memory| {
            let task = task(memory);
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }), spawn_type) {
            error!("Error while spawning WebAssembly process - {}", err);
        }
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    #[cfg(feature = "js")]
    fn spawn_wasm<F, Fut>(&self, task: F, spawn_type: SpawnType) -> AsyncResult<Fut::Output>
    where
        F: FnOnce(Option<VMMemory>) -> Fut,
        F: 'static,
        Fut: Future + 'static,
        Fut::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        if let Err(err) = self.task_wasm(Box::new(move |memory| {
            let task = task(memory);
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }), spawn_type) {
            error!("Error while spawning WebAssembly process - {}", err);
        }
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    fn spawn_dedicated<F>(&self, task: F)
    where
        F: FnOnce(),
        F: Send + 'static,
    {
        self.task_dedicated(Box::new(move || {
            task();
        }));
    }

    fn spawn_dedicated_async<F, Fut>(&self, task: F) -> AsyncResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_dedicated_async(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let ret = task.await;
                let _ = tx_result.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_result)
    }

    fn fork_shared<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        F: Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: Send,
    {
        self.task_shared(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let _ = task.await;
            })
        }));
    }

    fn fire_and_forget<T: Send + 'static>(&self, sender: &mpsc::Sender<T>, msg: T) {
        if let Err(mpsc::error::TrySendError::Full(msg)) = sender.try_send(msg) {
            let sender = sender.clone();
            self.task_shared(Box::new(move || {
                Box::pin(async move {
                    let _ = sender.send(msg).await;
                })
            }));
        }
    }

    fn fork_wasm<F, Fut>(&self, task: F, spawn_type: SpawnType)
    where
        F: FnOnce(Option<VMMemory>) -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send
    {
        self.spawn_wasm(task, spawn_type);
    }

    fn fork_dedicated<F>(&self, task: F)
    where
        F: FnOnce(),
        F: Send + 'static,
    {
        self.task_dedicated(Box::new(move || {
            task();
        }));
    }

    fn fork_dedicated_async<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut,
        F: Send + 'static,
        Fut: Future + 'static,
    {
        self.task_dedicated_async(Box::new(move || {
            let task = task();
            Box::pin(async move {
                let _ = task.await;
            })
        }));
    }

    fn fork_local<F>(&self, task: F)
    where
        F: Future + 'static,
    {
        self.task_local(Box::pin(async move {
            let _ = task.await;
        }))
    }
}
