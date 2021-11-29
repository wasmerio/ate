use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::*;

// This ABI implements a number of low level operating system
// functions that this terminal depends upon
#[async_trait]
pub trait SystemAbi
{
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>);

    fn spawn_blocking(&self, task: Box<dyn FnOnce() + Send + 'static>);

    fn spawn_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>);
    
    fn sleep(&self, ms: i32) -> Pin<Box<dyn Future<Output=()>>>;

    fn fetch_file(
        &self,
        path: &str,
    ) -> Pin<Box<dyn Future<Output=Result<Vec<u8>, i32>>>>;

    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output=Result<ReqwestResponse, i32>>>>;

    fn web_socket(&self, url: &str) -> Result<Arc<dyn WebSocketAbi>, String>;
}

// System call extensions that provide generics
pub(crate) trait SystemAbiExt
{
    fn spawn_task<Fut>(&self, future: Fut)
    where Fut: Future<Output = ()> + Send + 'static;

    fn spawn_local_task<F>(&self, task: F)
    where F: Future<Output = ()> + 'static;

    fn spawn_blocking_task<F>(&self, task: F)
    where F: FnOnce() + Send + 'static;
}

impl SystemAbiExt
for dyn SystemAbi
{
    fn spawn_task<Fut>(&self, future: Fut)
    where Fut: Future<Output = ()> + Send + 'static {
        self.spawn(Box::pin(future))
    }

    fn spawn_blocking_task<F>(&self, task: F)
    where F: FnOnce() + Send + 'static {
        self.spawn_blocking(Box::new(task))
    }

    fn spawn_local_task<F>(&self, task: F)
    where F: Future<Output = ()> + 'static {
        self.spawn_local(Box::pin(task))
    }
}