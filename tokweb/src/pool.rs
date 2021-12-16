#![allow(unused_imports)]
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use derivative::*;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::ops::Deref;
use std::ops::DerefMut;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Mutex;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Semaphore;

use js_sys::{JsString, Promise};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use term_lib::api::ThreadLocal;
use term_lib::common::MAX_MPSC;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DedicatedWorkerGlobalScope, WorkerOptions, WorkerType};
use xterm_js_rs::Terminal;

use super::common::*;
use super::fd::*;
use super::interval::*;
use super::tty::Tty;

pub type BoxRun<'a, T> =
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = T> + 'static>> + Send + 'a>;

pub type BoxRunWithThreadLocal<'a, T> = Box<
    dyn FnOnce(Rc<RefCell<ThreadLocal>>) -> Pin<Box<dyn Future<Output = T> + 'static>> + Send + 'a,
>;

trait AssertSendSync: Send + Sync {}
impl AssertSendSync for WebThreadPool {}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WebThreadPool {
    pool_reactors: Arc<PoolState>,
    pool_stateful: Arc<PoolState>,
    pool_dedicated: Arc<PoolState>,
}

enum Message {
    Run(BoxRun<'static, ()>),
    RunWithThreadLocal(BoxRunWithThreadLocal<'static, ()>),
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Run(_) => write!(f, "run-shared"),
            Message::RunWithThreadLocal(_) => write!(f, "run-dedicated"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PoolType {
    Shared,
    Stateful,
    Dedicated,
}

struct IdleThread {
    idx: usize,
    work: mpsc::Sender<Message>,
}

impl IdleThread {
    fn consume(self, msg: Message) {
        let _ = self.work.blocking_send(msg);
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PoolState {
    #[derivative(Debug = "ignore")]
    idle_rx: Mutex<mpsc::Receiver<IdleThread>>,
    idle_tx: mpsc::Sender<IdleThread>,
    idx_seed: AtomicUsize,
    idle_size: usize,
    blocking: bool,
    spawn: mpsc::Sender<Message>,
    #[allow(dead_code)]
    type_: PoolType,
}

pub struct ThreadState {
    pool: Arc<PoolState>,
    #[allow(dead_code)]
    idx: usize,
    tx: mpsc::Sender<Message>,
    rx: Mutex<Option<mpsc::Receiver<Message>>>,
    init: Mutex<Option<Message>>,
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

impl WebThreadPool {
    pub fn new(size: usize) -> Result<WebThreadPool, JsValue> {
        info!("pool::create(size={})", size);

        let (idle_tx1, idle_rx1) = mpsc::channel(MAX_MPSC);
        let (idle_tx2, idle_rx2) = mpsc::channel(MAX_MPSC);
        let (idle_tx3, idle_rx3) = mpsc::channel(MAX_MPSC);

        let (spawn_tx1, mut spawn_rx1) = mpsc::channel(MAX_MPSC);
        let (spawn_tx2, mut spawn_rx2) = mpsc::channel(MAX_MPSC);
        let (spawn_tx3, mut spawn_rx3) = mpsc::channel(MAX_MPSC);

        let pool_reactors = Arc::new(PoolState {
            idle_rx: Mutex::new(idle_rx1),
            idle_tx: idle_tx1,
            idx_seed: AtomicUsize::new(0),
            blocking: false,
            idle_size: 2usize.max(size),
            type_: PoolType::Shared,
            spawn: spawn_tx1,
        });

        let pool_stateful = Arc::new(PoolState {
            idle_rx: Mutex::new(idle_rx2),
            idle_tx: idle_tx2,
            idx_seed: AtomicUsize::new(0),
            blocking: true,
            idle_size: 2usize.max(size),
            type_: PoolType::Stateful,
            spawn: spawn_tx2,
        });

        let pool_dedicated = Arc::new(PoolState {
            idle_rx: Mutex::new(idle_rx3),
            idle_tx: idle_tx3,
            idx_seed: AtomicUsize::new(0),
            blocking: true,
            idle_size: 1usize.max(size),
            type_: PoolType::Dedicated,
            spawn: spawn_tx3,
        });

        let pool1 = pool_reactors.clone();
        let pool2 = pool_stateful.clone();
        let pool3 = pool_dedicated.clone();

        // The management thread will spawn other threads - this thread is safe from
        // being blocked by other thrads
        wasm_bindgen_futures::spawn_local(
            async move {
                loop {
                    select! {
                        spawn = spawn_rx1.recv() => {
                            if let Some(spawn) = spawn { pool1.expand(spawn); } else { break; }
                        }
                        spawn = spawn_rx2.recv() => {
                            if let Some(spawn) = spawn { pool2.expand(spawn); } else { break; }
                        }
                        spawn = spawn_rx3.recv() => {
                            if let Some(spawn) = spawn { pool3.expand(spawn); } else { break; }
                        }
                    }
                }
            }
        );

        let pool = WebThreadPool {
            pool_reactors,
            pool_stateful,
            pool_dedicated,
        };

        Ok(pool)
    }

    pub fn new_with_max_threads() -> Result<WebThreadPool, JsValue> {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = navigator, js_name = hardwareConcurrency)]
            static HARDWARE_CONCURRENCY: usize;
        }
        let pool_size = std::cmp::max(*HARDWARE_CONCURRENCY, 1);
        debug!("pool::max_threads={}", pool_size);
        Self::new(pool_size)
    }

    pub fn spawn_shared(&self, task: BoxRun<'static, ()>) {
        self.pool_reactors.spawn(Message::Run(task));
    }

    pub fn spawn_stateful(&self, task: BoxRunWithThreadLocal<'static, ()>) {
        self.pool_stateful.spawn(Message::RunWithThreadLocal(task));
    }

    pub fn spawn_dedicated(&self, task: BoxRun<'static, ()>) {
        self.pool_dedicated.spawn(Message::Run(task));
    }
}

impl PoolState {
    fn spawn(self: &Arc<Self>, mut msg: Message) {
        for _ in 0..10 {
            let guard = {
                self.idle_rx.try_lock().ok().map(|mut idle_rx| {
                    idle_rx.try_recv().ok()
                })
            };
            if let Some(thread) = guard {
                if let Some(thread) = thread {
                    thread.consume(msg);
                    return;
                }
                break;    
            }
            std::thread::yield_now();
        }

        for _ in 0..100 {
            match self.spawn.try_send(msg) {
                Ok(_) => { return; }
                Err(mpsc::error::TrySendError::Closed(_)) => { return; }
                Err(mpsc::error::TrySendError::Full(m)) => { msg = m; }
            }
            std::thread::yield_now();
        }

        if crate::common::is_worker() {
            let _ = self.spawn.blocking_send(msg);
        } else {
            let spawn = self.spawn.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = spawn.send(msg).await;
            });
        }
    }

    fn expand(self: &Arc<Self>, init: Message) {
        let (tx, rx) = mpsc::channel(MAX_MPSC);
        let idx = self.idx_seed.fetch_add(1usize, Ordering::Release);
        let state = Arc::new(ThreadState {
            pool: Arc::clone(self),
            idx,
            tx,
            rx: Mutex::new(Some(rx)),
            init: Mutex::new(Some(init)),
        });
        Self::start_worker_now(idx, state, None);
    }

    pub fn start_worker_now(
        idx: usize,
        state: Arc<ThreadState>,
        should_warn_on_error: Option<Terminal>,
    ) {
        let mut opts = WorkerOptions::new();
        opts.type_(WorkerType::Module);
        opts.name(&*format!("Worker-{:?}-{}", state.pool.type_, idx));

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
                let err = err
                    .as_string()
                    .unwrap_or_else(|| "unknown error".to_string());
                error!("failed to start worker thread - {}", err);

                if let Some(term) = should_warn_on_error {
                    term.write(
                        Tty::BAD_WORKER
                            .replace("\n", "\r\n")
                            .replace("\\x1B", "\x1B")
                            .replace("{error}", err.as_str())
                            .as_str(),
                    );
                }

                return;
            }
        });
    }
}

impl ThreadState {
    fn work(state: Arc<ThreadState>) {
        let thread_index = state.idx;
        info!(
            "worker started (index={}, type={:?})",
            thread_index, state.pool.type_
        );

        // Load the work queue receiver where other people will
        // send us the work that needs to be done
        let mut work_rx = {
            let mut lock = state.rx.lock().unwrap();
            lock.take().unwrap()
        };

        // Load the initial work
        let mut work = {
            let mut lock = state.init.lock().unwrap();
            lock.take()
        };

        // The work is done in an asynchronous engine (that supports Javascript)
        let thread_local = Rc::new(RefCell::new(ThreadLocal::default()));
        let work_tx = state.tx.clone();
        let pool = Arc::clone(&state.pool);
        let driver = async move {
            let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

            loop {
                // Process work until we need to go idle
                while let Some(task) = work {
                    match task {
                        Message::RunWithThreadLocal(task) => {
                            let thread_local = thread_local.clone();
                            if pool.blocking {
                                task(thread_local).await;
                            } else {
                                wasm_bindgen_futures::spawn_local(async move {
                                    task(thread_local).await;
                                });
                            }
                        }
                        Message::Run(task) => {
                            let future = task();
                            if pool.blocking {
                                future.await;
                            } else {
                                wasm_bindgen_futures::spawn_local(async move {
                                    future.await;
                                });
                            }
                        }
                    }

                    // Grab the next work
                    work = work_rx.try_recv().ok();
                }

                // If there iss already an idle thread thats older then
                // keep that one (otherwise ditch it) - this creates negative
                // pressure on the pool size.
                // The reason we keep older threads is to maximize cache hits such
                // as module compile caches.
                if let Ok(mut lock) = state.pool.idle_rx.try_lock() {
                    let mut others = Vec::new();
                    while let Ok(other) = lock.try_recv() {
                        others.push(other);
                    }

                    // Sort them in the order of index (so older ones come first)
                    others.sort_by_key(|k| k.idx);

                    // If the number of others (plus us) exceeds the maximum then
                    // we either drop ourselves or one of the others
                    if others.len() + 1 > pool.idle_size {
                        // How many are there already there that have a lower index - are we the one without a chair?
                        let existing = others
                            .iter()
                            .map(|a| a.idx)
                            .filter(|a| *a < thread_index)
                            .count();
                        if existing >= pool.idle_size {
                            for other in others {
                                let _ = state.pool.idle_tx.send(other).await;
                            }
                            info!(
                                "worker closed (index={}, type={:?})",
                                thread_index, pool.type_
                            );
                            break;
                        } else {
                            // Someone else is the one (the last one)
                            let leftover_chairs = others.len() - 1;
                            for other in others.into_iter().take(leftover_chairs) {
                                let _ = state.pool.idle_tx.send(other).await;
                            }
                        }
                    } else {
                        // Add them all back in again (but in the right order)
                        for other in others {
                            let _ = state.pool.idle_tx.send(other).await;
                        }
                    }
                }

                // Now register ourselves as idle
                trace!(
                    "pool is idle (thread_index={}, type={:?})",
                    thread_index,
                    pool.type_
                );
                let idle = IdleThread {
                    idx: thread_index,
                    work: work_tx.clone(),
                };
                if let Err(_) = state.pool.idle_tx.send(idle).await {
                    info!(
                        "pool is closed (thread_index={}, type={:?})",
                        thread_index, pool.type_
                    );
                    break;
                }

                // Do a blocking recv (if this fails the thread is closed)
                work = match work_rx.recv().await {
                    Some(a) => Some(a),
                    None => {
                        info!(
                            "worker closed (index={}, type={:?})",
                            thread_index, pool.type_
                        );
                        break;
                    }
                };
            }

            global.close();
        };
        wasm_bindgen_futures::spawn_local(driver);
    }
}

#[wasm_bindgen(skip_typescript)]
pub fn worker_entry_point(state_ptr: u32) {
    let state = unsafe { Arc::<ThreadState>::from_raw(state_ptr as *const ThreadState) };

    let name = js_sys::global()
        .unchecked_into::<DedicatedWorkerGlobalScope>()
        .name();
    debug!("{}: Entry", name);
    ThreadState::work(state);
}
