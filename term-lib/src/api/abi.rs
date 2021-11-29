use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::*;

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
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    );

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated(&self, task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>);

    /// Starts an asynchronous task on the current thread. This is useful for
    /// launching background work with variables that are not Send.
    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>);

    /// Puts the current thread to sleep for a fixed number of milliseconds
    fn sleep(&self, ms: i32) -> Pin<Box<dyn Future<Output = ()>>>;

    /// Fetches a data file from the local context of the process
    fn fetch_file(&self, path: &str) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, i32>>>>;

    /// Performs a HTTP or HTTPS request to a destination URL
    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<ReqwestResponse, i32>>>>;

    fn web_socket(&self, url: &str) -> Result<Arc<dyn WebSocketAbi>, String>;
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
        Fut: Future + 'static,
        Fut::Output: Send;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// The return value of the spawned thread can be read either synchronously
    /// or asynchronously
    fn spawn_dedicated<F>(&self, task: F) -> AsyncResult<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send;

    // Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    /// This is the fire-and-forget variet of spawning background work
    fn fork_shared<F>(&self, task: F)
    where
        F: Future + Send + 'static;

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    /// This is the fire-and-forget variet of spawning background work
    fn fork_dedicated<F>(&self, task: F)
    where
        F: Future + Send + 'static;

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
        Fut: Future + 'static,
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
        AsyncResult::new(rx_result)
    }

    fn spawn_dedicated<F>(&self, task: F) -> AsyncResult<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        let (tx_result, rx_result) = mpsc::channel(1);
        self.task_dedicated(Box::pin(async move {
            let ret = task.await;
            let _ = tx_result.send(ret).await;
        }));
        AsyncResult::new(rx_result)
    }

    fn fork_shared<F>(&self, task: F)
    where
        F: Future + Send + 'static,
    {
        self.task_shared(Box::new(move || {
            Box::pin(async move {
                let _ = task.await;
            })
        }));
    }

    fn fork_dedicated<F>(&self, task: F)
    where
        F: Future + Send + 'static,
    {
        self.task_dedicated(Box::pin(async move {
            let _ = task.await;
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
