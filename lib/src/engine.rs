#![allow(unused_imports)]
use std::sync::Arc;
use std::rc::Rc;
use std::future::Future;
use tokio::runtime::Runtime;
use tokio::task::LocalSet;
use tokio::task::JoinHandle;
use parking_lot::Mutex;
use tokio::sync::mpsc;
use std::pin::Pin;
use tokio::select;
use once_cell::sync::Lazy;

pub struct TaskEngine
where Self: Send + Sync + 'static
{
    #[cfg(not(feature = "enable_mt"))]
    tx: mpsc::Sender<Pin<Box<dyn Future<Output=()> + Send + 'static>>>,
    #[cfg(not(feature = "enable_mt"))]
    rx: Mutex<Option<mpsc::Receiver<Pin<Box<dyn Future<Output=()> + Send + 'static>>>>>,
}

static GLOBAL_ENGINE: Lazy<TaskEngine> = Lazy::new(|| {
    TaskEngine::new_static()
});

impl TaskEngine
{
    fn new_static() -> TaskEngine
    {
        #[cfg(not(feature = "enable_mt"))]
        let (tx, rx) = mpsc::channel(100);

        TaskEngine {
            #[cfg(not(feature = "enable_mt"))]
            tx,
            #[cfg(not(feature = "enable_mt"))]
            rx: Mutex::new(Some(rx)),
        }
    }

    async fn aggregated_run<T>(future: T)
    where T: Future + Send + 'static,
          T::Output: Send + 'static,
    {
        let rx = {
            let mut guard = GLOBAL_ENGINE.rx.lock();
            guard.take()
        };
        if let Some(mut rx) = rx {
            let (exit_tx, mut exit_rx) = mpsc::channel(1);

            let local = tokio::task::LocalSet::new();
            local.spawn_local(async move {
                let ret = future.await;
                exit_tx.send(()).await.expect("Failed to exit the main thread");
                ret
            });
            loop {
                let tick = async {
                    select! {
                        a = rx.recv() => {
                            match a {
                                Some(f) => {
                                    local.spawn_local(f);
                                    false
                                },
                                None => true
                            }
                        },
                        _ = exit_rx.recv() => {
                            true
                        }
                    }
                };
                let exit = local.run_until(tick).await;
                if exit {
                    break;
                }
            }
        }
    }

    pub async fn run_until<T>(future: T)
    where T: Future + Send + 'static,
          T::Output: Send + 'static,
    {
        if cfg!(feature="enable_mt") {
            future.await;
        } else if cfg!(feature="enable_web") {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(TaskEngine::aggregated_run(future))
        } else {
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(TaskEngine::aggregated_run(future))
            }).await.unwrap()
        }
    }

    #[cfg(not(feature = "enable_mt"))]
    pub async fn spawn<T>(future: T)
    where T: Future + Send + 'static,
          T::Output: Send + 'static,
    {
        match GLOBAL_ENGINE.tx.send(Box::pin(async {
            future.await;
        })).await {
            Ok(a) => a,
            Err(err) => {
                panic!("Failed to spawn a background thread - {}", err.to_string());
            }
        }
    }

    #[cfg(feature = "enable_mt")]
    pub async fn spawn<T>(future: T)
    where T: Future + Send + 'static,
          T::Output: Send + 'static,
    {
        tokio::spawn(future).await;
    }
}