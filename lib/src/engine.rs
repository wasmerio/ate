#![allow(unused_imports)]
use cooked_waker::*;
use fxhash::FxHashMap;
use once_cell::sync::Lazy;
use pin_project_lite::pin_project;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::atomic::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::*;
use std::thread::AccessError;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

pub struct TaskEngine {}

impl TaskEngine {
    pub async fn run_until<F>(future: F) -> F::Output
    where
        F: Future,
    {
        future.await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn<T>(task: T) -> tokio::task::JoinHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(task)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn<T>(task: T)
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        wasm_bus::task::spawn(task)
    }

    pub async fn spawn_blocking<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let ret = tokio::task::spawn_blocking(f).await;
        ret.unwrap()
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
    wasm_bus::time::sleep(duration).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn timeout<T>(duration: Duration, future: T) -> Result<T::Output, wasm_bus::time::Elapsed>
where
    T: Future,
{
    wasm_bus::time::timeout(duration, future).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn timeout<T>(duration: Duration, future: T) -> tokio::time::Timeout<T>
where
    T: Future,
{
    tokio::time::timeout(duration, future)
}
