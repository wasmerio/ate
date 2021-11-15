#![allow(unused_imports)]
#![allow(dead_code)]
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use std::borrow::Borrow;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Semaphore;

use js_sys::{JsString, Promise};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DedicatedWorkerGlobalScope, WorkerOptions, WorkerType};
use xterm_js_rs::{Terminal};

use super::common::*;
use super::interval::*;
use super::fd::*;
use super::tty::Tty;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type BoxTask<'a, T> = Box<dyn FnOnce() -> T + Send + 'a>;

trait AssertSendSync: Send + Sync {}
impl AssertSendSync for ThreadPool {}

#[wasm_bindgen]
pub struct ThreadPool {
    pool_reactors: Arc<PoolState>,
    pool_blocking: Arc<PoolState>,
    manager: Arc<LeakyInterval>,
}

enum Message {
    RunAsync(BoxFuture<'static, ()>),
    Run(BoxTask<'static, ()>),
    Close,
}

#[derive(Debug)]
enum PoolType {
    Reactor,
    Thread,
}

pub struct PoolState {
    tx: broadcast::Sender<()>,
    ref_cnt: AtomicUsize,
    queue: Mutex<VecDeque<Message>>,
    size: AtomicUsize,
    idle: AtomicUsize,
    starting: AtomicUsize,
    id_seed: AtomicUsize,
    min_size: usize,
    max_size: usize,
    type_: PoolType,
}

pub struct ThreadState {
    pool: Arc<PoolState>,
    idx: usize,
}

impl Clone for ThreadPool {
    fn clone(&self) -> Self {
        self.pool_reactors.ref_cnt.fetch_add(1, Ordering::Relaxed);
        self.pool_blocking.ref_cnt.fetch_add(1, Ordering::Relaxed);
        Self {
            pool_reactors: self.pool_reactors.clone(),
            pool_blocking: self.pool_blocking.clone(),
            manager: self.manager.clone(),
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        if self.pool_reactors.ref_cnt.fetch_sub(1, Ordering::Relaxed) == 1 {
            for _ in 0..self.pool_reactors.size.load(Ordering::Relaxed) {
                self.pool_reactors.send(Message::Close);
            }
        }
        if self.pool_blocking.ref_cnt.fetch_sub(1, Ordering::Relaxed) == 1 {
            for _ in 0..self.pool_blocking.size.load(Ordering::Relaxed) {
                self.pool_blocking.send(Message::Close);
            }
        }
    }
}

#[wasm_bindgen]
pub struct LoaderHelper {}
#[wasm_bindgen]
impl LoaderHelper {
    #[wasm_bindgen(js_name = mainJS)]
    pub fn main_js(&self) -> JsString {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["import", "meta"], js_name = url)]
            static URL: JsString;
        }

        URL.clone()
    }
}

#[wasm_bindgen(module = "/public/worker.js")]
extern "C" {
    #[wasm_bindgen(js_name = "startWorker")]
    fn start_worker(
        module: JsValue,
        memory: JsValue,
        shared_data: JsValue,
        opts: WorkerOptions,
        builder: LoaderHelper,
    ) -> Promise;
}

impl ThreadPool {
    pub fn new(size: usize, terminal: Terminal) -> Result<ThreadPool, JsValue> {
        info!("pool::create(size={})", size);

        let (tx1, _) = broadcast::channel(64);
        let (tx2, _) = broadcast::channel(64);

        let pool_reactors = Arc::new(PoolState {
            tx: tx1,
            ref_cnt: AtomicUsize::new(1),
            size: AtomicUsize::new(0),
            id_seed: AtomicUsize::new(0),
            queue: Mutex::new(VecDeque::new()),
            idle: AtomicUsize::new(0),
            starting: AtomicUsize::new(0),
            min_size: 1,
            max_size: size,
            type_: PoolType::Reactor,
        });

        let pool_blocking = Arc::new(PoolState {
            tx: tx2,
            ref_cnt: AtomicUsize::new(1),
            size: AtomicUsize::new(0),
            id_seed: AtomicUsize::new(0),
            queue: Mutex::new(VecDeque::new()),
            idle: AtomicUsize::new(0),
            starting: AtomicUsize::new(0),
            min_size: 1,
            max_size: 1000usize,
            type_: PoolType::Thread,
        });

        let manager = {
            let pool_blocking = Arc::downgrade(&pool_blocking);
            let pool_reactors = Arc::downgrade(&pool_reactors);
            LeakyInterval::new(std::time::Duration::from_millis(50), move || {
                if let Some(pool) = pool_blocking.upgrade() {
                    pool.manage();
                }
                if let Some(pool) = pool_reactors.upgrade() {
                    pool.manage();
                }
            })
        };

        let pool = ThreadPool {
            pool_reactors,
            pool_blocking,
            manager: Arc::new(manager),
        };

        pool.pool_reactors.expand_now(Some(terminal));
        pool.pool_blocking.expand_now(None);

        Ok(pool)
    }

    pub fn new_with_max_threads(terminal: Terminal) -> Result<ThreadPool, JsValue> {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = navigator, js_name = hardwareConcurrency)]
            static HARDWARE_CONCURRENCY: usize;
        }
        let pool_size = std::cmp::max(*HARDWARE_CONCURRENCY, 1);
        debug!("pool::max_threads={}", pool_size);
        Self::new(pool_size, terminal)
    }

    pub fn spawn<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.pool_reactors.send(Message::RunAsync(Box::pin(future)));
    }

    pub fn spawn_blocking<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool_blocking.send(Message::Run(Box::new(task)));
    }
}

impl PoolState {
    pub fn expand_now(self: &Arc<PoolState>, should_warn_on_error: Option<Terminal>) {
        let pool = self.clone();

        let idx = pool.id_seed.fetch_add(1, Ordering::Relaxed);
        pool.starting.fetch_add(1, Ordering::Relaxed);

        let state = Arc::new(ThreadState { pool: pool, idx });
        Self::start_worker_now(idx, state, should_warn_on_error);
    }

    pub fn start_worker_now(idx: usize, state: Arc<ThreadState>, should_warn_on_error: Option<Terminal>) {
        let mut opts = WorkerOptions::new();
        opts.type_(WorkerType::Module);
        opts.name(&*format!("Worker-{}", idx));

        let ptr = Arc::into_raw(state);

        let result = wasm_bindgen_futures::JsFuture::from(start_worker(
            wasm_bindgen::module(),
            wasm_bindgen::memory(),
            JsValue::from(ptr as u32),
            opts,
            LoaderHelper {},
        ));

        wasm_bindgen_futures::spawn_local(async move {
            let ret = result.await;
            if let Err(err) = ret {
                let err = err.as_string().unwrap_or_else(|| "unknown error".to_string());
                error!("failed to start worker thread - {}", err);

                if let Some(term) = should_warn_on_error {
                    term.write(Tty::BAD_WORKER
                        .replace("\n", "\r\n")
                        .replace("\\x1B", "\x1B")
                        .replace("{error}", err.as_str())
                        .as_str());
                }

                return;
            }
            info!("worker thread spawned - index={}", idx);
        });
    }

    pub fn shrink(self: &Arc<PoolState>) {
        self.send(Message::Close);
    }

    pub fn manage(self: &Arc<PoolState>) {
        let size = self.size.load(Ordering::Relaxed);
        let idle = self.idle.load(Ordering::Relaxed);
        let starting = self.starting.load(Ordering::Relaxed);
        let backlog = self.queue.lock().unwrap().len();

        if backlog >= starting + idle {
            if size < self.max_size {
                self.expand_now(None);
            }
        } else if backlog <= 0 && idle <= 0 {
            if size > self.min_size {
                self.expand_now(None);
            }
        }
    }

    fn send(&self, msg: Message) {
        {
            let mut queue = self.queue.lock().unwrap();
            queue.push_back(msg);
        }
        let _ = self.tx.send(());
    }
}

impl ThreadState {
    fn work(state: Arc<ThreadState>) {
        let mut rx = state.pool.tx.subscribe();
        let pool = Arc::clone(&state.pool);
        let driver = async move {
            let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

            pool.size.fetch_add(1, Ordering::Relaxed);
            pool.idle.fetch_add(1, Ordering::Relaxed);
            pool.starting.fetch_sub(1, Ordering::Relaxed);

            loop {
                let msg = {
                    let mut queue = pool.queue.lock().unwrap();
                    queue.pop_front()
                };
                if let Some(msg) = msg {
                    match msg {
                        Message::Run(task) => {
                            pool.idle.fetch_sub(1, Ordering::Relaxed);
                            task();
                            pool.idle.fetch_add(1, Ordering::Relaxed);
                        }
                        Message::RunAsync(future) => wasm_bindgen_futures::spawn_local(future),
                        Message::Close => {
                            debug!("pool - thread closed");
                            break;
                        }
                    }
                }

                let _ = rx.recv().await;
            }
            info!("{}: Shutting down", global.name());

            pool.idle.fetch_sub(1, Ordering::Relaxed);
            pool.size.fetch_sub(1, Ordering::Relaxed);
            global.close();
        };
        wasm_bindgen_futures::spawn_local(driver);
    }
}

#[wasm_bindgen(skip_typescript)]
pub fn worker_entry_point(state_ptr: u32) {
    info!("worker started");
    let state = unsafe { Arc::<ThreadState>::from_raw(state_ptr as *const ThreadState) };

    let name = js_sys::global()
        .unchecked_into::<DedicatedWorkerGlobalScope>()
        .name();
    debug!("{}: Entry", name);
    ThreadState::work(state);
}
