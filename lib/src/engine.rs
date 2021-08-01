#![allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use std::ops::DerefMut;
use std::sync::Arc;
use std::future::Future;
use std::task::*;
use std::pin::Pin;
use std::cell::RefCell;
use pin_project_lite::pin_project;
use std::sync::atomic::*;
use cooked_waker::*;
use fxhash::FxHashMap;
use std::thread::AccessError;

#[cfg(not(feature = "enable_mt"))]
type Task = Pin<Box<dyn Future<Output=()>>>;

pub struct TaskEngine
{
    #[cfg(not(feature = "enable_mt"))]
    tasks: Vec<Task>,
}

pin_project! {
    #[derive(Debug)]
    struct RunUntil<T>
    {
        #[pin]
        future: T
    }
}

impl<T> Future
for RunUntil<T>
where T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let f = self.project().future;
        let status = f.poll(cx);
        #[cfg(not(feature = "enable_mt"))]
        TaskEngine::process(cx);
        status
    }
}

pub struct TickTask
{
}

impl Future
for TickTask
{
    type Output = ();

    #[allow(unused_variables)]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        #[cfg(not(feature = "enable_mt"))]
        TaskEngine::process(cx);
        Poll::Ready(())
    }
}

impl TaskEngine
{
    #[cfg(not(feature = "enable_mt"))]
    thread_local! {
        static LOCAL: RefCell<TaskEngine> = RefCell::new(
            TaskEngine {
                tasks: Vec::new(),
            }
        );
    }

    pub async fn run_until<F>(future: F) -> F::Output
    where F: Future,
    {
        RunUntil {
            future,
        }.await
    }

    pub async fn tick() {
        TickTask {
        }.await
    }

    #[cfg(not(feature = "enable_mt"))]
    fn process(cx: &mut std::task::Context<'_>)
    {
        let mut again = Vec::new();
        while let Some(mut task) = TaskEngine::instance(|e| e.tasks.pop()).ok().flatten() {
            let f = task.as_mut();
            if let Poll::Pending = f.poll(cx) {
                again.push(task);
            }
        }
        TaskEngine::instance(|e| e.tasks.append(&mut again)).unwrap();
    }

    #[cfg(not(feature = "enable_mt"))]
    pub fn spawn<T>(task: T)
    where T: Future + 'static,
    {
        let task = Box::pin(async { task.await; });
        TaskEngine::instance(|e| e.tasks.push(task)).unwrap();
    }

    #[cfg(feature = "enable_mt")]
    pub fn spawn<T>(task: T)
    where T: Future + Send + 'static,
          T::Output: Send + 'static,
    {
        tokio::spawn(task);
    }

    #[cfg(not(feature = "enable_mt"))]
    pub fn instance<F, R>(f: F) -> Result<R, AccessError>
    where F: FnOnce(&mut TaskEngine) -> R
    {
        TaskEngine::LOCAL.try_with(|e| {
            let mut guard = e.borrow_mut();
            let e = guard.deref_mut();
            f(e)
        })
    }
}