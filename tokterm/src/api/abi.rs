use async_trait::async_trait;
use std::future::Future;

// This ABI implements a number of low level operating system
// functions that this terminal depends upon
#[async_trait]
pub trait SystemAbi
{
    fn spawn<Fut>(&self, future: Fut)
    where Fut: Future<Output = ()> + Send + 'static;

    fn spawn_local<F>(&self, task: F)
    where F: Future<Output = ()> + 'static;

    fn spawn_blocking<F>(&self, task: F)
    where F: FnOnce() + Send + 'static;

    async fn sleep(&self, ms: i32);
}