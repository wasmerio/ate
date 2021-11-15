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

#[cfg(not(feature = "enable_mt"))]
type Task = Pin<Box<dyn Future<Output = ()>>>;

#[no_mangle]
pub extern "C" fn __thread_entry() {
    TaskEngine::thread_entry();
}

pub struct BackgroundTask {
    work: Box<dyn FnOnce() + Send + 'static>,
}

pub struct BackgroundTaskPool {
    wake: broadcast::Sender<()>,
    jobs: Mutex<VecDeque<BackgroundTask>>,
}

#[derive(Default)]
pub struct PendingOnce {
    used: bool,
}

impl Future for PendingOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        if self.used == true {
            Poll::Ready(())
        } else {
            self.used = true;
            cx.waker().clone().wake();
            Poll::Pending
        }
    }
}

pub struct TaskEngine {
    #[cfg(not(feature = "enable_mt"))]
    tasks: VecDeque<Task>,
}

pin_project! {
    #[derive(Debug)]
    struct RunUntil<T>
    {
        #[pin]
        future: T
    }
}

impl<T> Future for RunUntil<T>
where
    T: Future,
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

pin_project! {
    #[derive(Debug)]
    pub struct Timeout<T> {
        #[pin]
        future: T,
        #[pin]
        duration: Duration,
        #[pin]
        start: Instant,
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Elapsed(Duration);

impl std::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<T> Future for Timeout<T>
where
    T: Future,
{
    type Output = Result<T::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let start = self.start.clone();
        let duration = self.duration.clone();

        let f = self.project().future;
        let status = f.poll(cx);
        if let Poll::Ready(v) = status {
            return Poll::Ready(Ok(v));
        }

        let time = Instant::now();
        let elapsed = time - start;
        if elapsed.ge(&duration) {
            return Poll::Ready(Err(Elapsed(elapsed)));
        }

        #[cfg(not(feature = "enable_mt"))]
        if TaskEngine::process(cx) <= 0 {
            std::thread::yield_now();
        }

        #[cfg(feature = "enable_mt")]
        std::thread::yield_now();

        cx.waker().clone().wake();
        Poll::Pending
    }
}

pub struct TickTask {
    #[allow(dead_code)]
    idle: bool,
}

impl Future for TickTask {
    type Output = ();

    #[allow(unused_variables)]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        #[cfg(not(feature = "enable_mt"))]
        {
            let cnt = TaskEngine::process(cx);
            if self.idle && cnt <= 0 {
                std::thread::yield_now();
            }
        }
        Poll::Ready(())
    }
}

static POOL: Lazy<BackgroundTaskPool> = Lazy::new(|| {
    let (tx_wake, _) = broadcast::channel(100);
    BackgroundTaskPool {
        wake: tx_wake,
        jobs: Mutex::new(VecDeque::new()),
    }
});

impl TaskEngine {
    #[cfg(not(feature = "enable_mt"))]
    thread_local! {
        static LOCAL: RefCell<TaskEngine> = RefCell::new(
            TaskEngine {
                tasks: VecDeque::new(),
            }
        );
    }

    pub async fn run_until<F>(future: F) -> F::Output
    where
        F: Future,
    {
        RunUntil { future }.await
    }

    pub async fn tick(idle: bool) {
        if idle {
            PendingOnce::default().await
        }
        TickTask { idle }.await
    }

    #[cfg(not(feature = "enable_mt"))]
    fn process(cx: &mut std::task::Context<'_>) -> usize {
        let mut cnt = 0usize;
        let mut again = Vec::new();
        let mut again_back = Vec::new();
        while let Some(mut task) = TaskEngine::instance(|e| e.tasks.pop_front()).unwrap() {
            let f = task.as_mut();
            if let Poll::Pending = f.poll(cx) {
                if again_back.len() <= 0 {
                    again_back.push(task);
                } else {
                    again.push(task);
                }
            } else {
                cnt += 1;
            }
        }
        TaskEngine::instance(move |e| {
            for task in again {
                e.tasks.push_back(task);
            }
            for task in again_back {
                e.tasks.push_back(task);
            }
        })
        .unwrap();
        cnt
    }

    #[cfg(not(feature = "enable_mt"))]
    pub fn spawn<T>(task: T)
    where
        T: Future + 'static,
    {
        let task = Box::pin(async {
            task.await;
        });
        TaskEngine::instance(|e| e.tasks.push_back(task)).unwrap();
    }

    #[cfg(feature = "enable_mt")]
    pub fn spawn<T>(task: T) -> tokio::task::JoinHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(task)
    }

    #[cfg(all(feature = "enable_mt", feature = "enable_full"))]
    pub async fn spawn_blocking<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let ret = tokio::task::spawn_blocking(f).await;
        ret.unwrap()
    }

    #[cfg(all(feature = "enable_mt", not(feature = "enable_full")))]
    pub async fn spawn_blocking<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx_ret, rx_ret) = oneshot::channel();
        {
            let work = move || {
                let ret = f();
                let _ = tx_ret.send(ret);
            };

            let mut pool = POOL.jobs.lock();
            pool.push_back(BackgroundTask {
                work: Box::new(work),
            });
        }
        let _ = POOL.wake.send(());
        rx_ret.await.unwrap()
    }

    #[cfg(not(feature = "enable_mt"))]
    pub async fn spawn_blocking<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        f()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn thread_entry() {
        info!("background thread exited");
        let mut wake = POOL.wake.subscribe();
        loop {
            loop {
                let task = {
                    let mut pool = POOL.jobs.lock().unwrap();
                    pool.pop_front()
                };

                if let Some(task) = task {
                    (task.work)();
                } else {
                    break;
                }
            }

            if let Err(_err) = wake.recv().await {
                break;
            }
        }
        info!("background thread exited");
    }

    #[cfg(not(feature = "enable_mt"))]
    pub fn instance<F, R>(f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&mut TaskEngine) -> R,
    {
        TaskEngine::LOCAL.try_with(|e| {
            let mut guard = e.borrow_mut();
            let e = guard.deref_mut();
            f(e)
        })
    }
}

#[cfg(not(feature = "enable_mt"))]
pub async fn sleep(_duration: Duration) {
    TaskEngine::tick(true).await
}

#[cfg(feature = "enable_mt")]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(feature = "enable_mt")]
pub fn timeout<T>(duration: Duration, future: T) -> tokio::time::Timeout<T>
where
    T: Future,
{
    tokio::time::timeout(duration, future)
}

#[cfg(not(feature = "enable_mt"))]
pub fn timeout<T>(duration: Duration, future: T) -> Timeout<T>
where
    T: Future,
{
    Timeout {
        future,
        duration,
        start: Instant::now(),
    }
}
