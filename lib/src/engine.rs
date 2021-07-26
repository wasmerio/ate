#![allow(unused_imports)]
use log::{info, warn, debug, error};
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
    woken_tasks: Vec<Task>,
    #[cfg(not(feature = "enable_mt"))]
    stored_tasks: FxHashMap<u64, Task>,
}

#[cfg(not(feature = "enable_mt"))]
pin_project! {
    #[derive(Debug)]
    struct RunUntil<T>
    {
        #[pin]
        future: T
    }
}

#[cfg(not(feature = "enable_mt"))]
impl<T> Future
for RunUntil<T>
where T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let f = self.project().future;
        let status = f.poll(cx);
        TaskEngine::process(cx);
        status
    }
}

#[cfg(not(feature = "enable_mt"))]
pub struct ConcurrentTask
{
    task_id: u64,
}

#[cfg(not(feature = "enable_mt"))]
impl Future
for ConcurrentTask
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        if let Some(task) = TaskEngine::retrieve_task(self.task_id) {
            process_task(task, cx);
        }
        TaskEngine::process(cx);
        Poll::Ready(())
    }
}

#[cfg(not(feature = "enable_mt"))]
struct ConcurrentWaker
{
    state: AtomicU64,
    parent: Waker,
}

#[cfg(not(feature = "enable_mt"))]
impl WakeRef for ConcurrentWaker {
    fn wake_by_ref(&self) {
        let task_id = self.state.swap(u64::MAX, Ordering::SeqCst);
        if let Some(task) = TaskEngine::retrieve_task(task_id) {
            if let Ok(_) = TaskEngine::instance(|e| e.woken_tasks.push(task)) {
                self.parent.wake_by_ref();
            }
        }
    }
}

#[cfg(not(feature = "enable_mt"))]
impl ConcurrentWaker
{
    fn new(parent: Waker) -> Arc<ConcurrentWaker> {
        let waker = ConcurrentWaker {
            state: AtomicU64::new(0u64),
            parent,
        };
        Arc::new(waker)
    }

    fn stage(&self, task: Task) {
        let task_id = TaskEngine::store_task(task);
        if self.state.swap(task_id, Ordering::SeqCst) == u64::MAX {
            self.wake_by_ref();
        }
    }
}

#[cfg(not(feature = "enable_mt"))]
fn process_task(mut task: Task, cx: &mut std::task::Context<'_>) {
    let waker = ConcurrentWaker::new(cx.waker().clone());
    let cx_waker = Arc::clone(&waker).into_waker();
    let mut cx= futures::task::Context::from_waker(&cx_waker);
    
    let f = task.as_mut();
    if let Poll::Pending = f.poll(&mut cx) {
        waker.stage(task);
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
                woken_tasks: Vec::new(),
                stored_tasks: FxHashMap::default(),
            }
        );
    }

    #[cfg(not(feature = "enable_mt"))]
    pub(crate) async fn run_until<F>(future: F) -> F::Output
    where F: Future,
    {
        RunUntil {
            future,
        }.await
    }

    #[cfg(feature = "enable_mt")]
    pub(crate) async fn run_until<F>(future: F) -> F::Output
    where F: Future,
    {
        future.await
    }

    #[must_use = "tick operations do nothing unless you `.await` or poll them"]
    pub fn tick() -> TickTask {
        TickTask {
        }
    }

    #[cfg(not(feature = "enable_mt"))]
    fn process(cx: &mut std::task::Context<'_>)
    {
        while let Some(task) = TaskEngine::instance(|e| e.woken_tasks.pop()).ok().flatten() {
            process_task(task, cx);
        }
    }

    #[cfg(not(feature = "enable_mt"))]
    fn store_task(task: Task) -> u64 {
        TaskEngine::instance(|e| {
            let mut task_id = fastrand::u64(..);
            loop {
                if e.stored_tasks.contains_key(&task_id) == false {
                    e.stored_tasks.insert(task_id, task);
                    break;
                }
                task_id = fastrand::u64(..);
            }
            task_id
        }).ok().unwrap_or_else(|| 0u64)
    }

    #[cfg(not(feature = "enable_mt"))]
    fn retrieve_task(task_id: u64) -> Option<Task>
    {
        TaskEngine::instance(|e| {
            e.stored_tasks.remove(&task_id)
        }).ok().flatten()
    }

    #[must_use = "spawn operations do nothing unless you `.await` or poll them - it will not block the task"]
    #[cfg(not(feature = "enable_mt"))]
    pub fn spawn<T>(task: T) -> ConcurrentTask
    where T: Future + 'static,
    {
        let task_id = TaskEngine::store_task(Box::pin(async { task.await; }));
        ConcurrentTask {
            task_id
        }
    }

    #[cfg(feature = "enable_mt")]
    pub async fn spawn<T>(task: T)
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