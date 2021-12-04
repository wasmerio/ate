use std::future::Future;

use crate::rt::RUNTIME;
pub use crate::engine::ListenerBuilder;

pub fn block_on<F>(task: F) -> F::Output
where
    F: Future,
{
    RUNTIME.block_on(task)
}

pub fn spawn<F>(task: F)
where
    F: Future + Send + 'static,
{
    RUNTIME.spawn(task)
}

pub fn wake() {
    RUNTIME.wake();
}

pub fn serve()
{
    RUNTIME.serve();
}