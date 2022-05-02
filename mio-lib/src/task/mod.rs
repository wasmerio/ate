use std::future::Future;

#[cfg(target_os = "wasi")]
pub fn spawn<F>(task: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    wasm_bus::task::spawn(task);
}

#[cfg(not(target_os = "wasi"))]
pub fn spawn<F>(task: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::task::spawn(task);
}