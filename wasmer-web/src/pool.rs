#![allow(unused_imports)]
use bytes::Bytes;
use js_sys::Uint8Array;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use derivative::*;
use wasm_bindgen_futures::JsFuture;
use wasmer_os::api::SpawnType;
use wasmer_os::wasmer::MemoryType;
use wasmer_os::wasmer::Module;
use wasmer_os::wasmer::Store;
use wasmer_os::wasmer::WASM_MAX_PAGES;
use wasmer_os::wasmer::vm::VMMemory;
use wasmer_os::wasmer_wasi::WasiThreadError;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::ops::DerefMut;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU32;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Semaphore;
use once_cell::sync::Lazy;

use js_sys::{JsString, Promise};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use wasmer_os::api::ThreadLocal;
use wasmer_os::common::MAX_MPSC;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DedicatedWorkerGlobalScope, WorkerOptions, WorkerType};
use xterm_js_rs::Terminal;

use super::common::*;
use super::fd::*;
use super::interval::*;
use super::tty::Tty;

pub type BoxRun<'a> =
    Box<dyn FnOnce() + Send + 'a>;

pub type BoxRunAsync<'a, T> =
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = T> + 'static>> + Send + 'a>;

#[derive(Debug, Clone, Copy)]
enum WasmRunType {
    Create,
    CreateWithMemory(MemoryType),
    Existing(u32),
}

struct WasmRunCommand {
    run: Box<dyn FnOnce(Store, Module, Option<VMMemory>) -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ty: WasmRunType,
    store: Store,
    module_bytes: Bytes,
    free_memory: Arc<mpsc::Sender<u32>>,
}

enum WasmRunMemory {
    WithoutMemory,
    WithMemory(MemoryType)
}

struct WasmRunContext {
    id: u32,
    cmd: WasmRunCommand,
    memory: WasmRunMemory,
}

#[derive(Clone)]
struct WasmInstance
{
    ref_cnt: u32,
    module: js_sys::WebAssembly::Module,
    module_bytes: Bytes,
    memory: js_sys::WebAssembly::Memory,
    memory_type: MemoryType,
}

thread_local! {
    static THREAD_LOCAL_ROOT_WASM_INSTANCES: std::cell::RefCell<HashMap<u32, WasmInstance>> 
        = RefCell::new(HashMap::new());
    static THREAD_LOCAL_CURRENT_WASM: std::cell::RefCell<Option<u32>>
        = RefCell::new(None);
}
static WASM_SEED: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(1));

trait AssertSendSync: Send + Sync {}
impl AssertSendSync for WebThreadPool {}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WebThreadPool {
    pool_reactors: Arc<PoolState>,
    pool_dedicated: Arc<PoolState>,
    spawn_wasm: Arc<mpsc::Sender<WasmRunCommand>>,
    free_memory: Arc<mpsc::Sender<u32>>,
}

enum Message {
    Run(BoxRun<'static>),
    RunAsync(BoxRunAsync<'static, ()>),
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Run(_) => write!(f, "run"),
            Message::RunAsync(_) => write!(f, "run-async"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PoolType {
    Shared,
    Dedicated,
}

struct IdleThread {
    idx: usize,
    work: mpsc::Sender<Message>,
}

impl IdleThread {
    #[allow(dead_code)]
    fn consume(self, msg: Message) {
        let _ = self.work.blocking_send(msg);
    }

    fn try_consume(self, msg: Message) -> Result<(), (IdleThread, Message)> {
        match self.work.try_send(msg) {
            Ok(_) => Ok(()),
            Err(mpsc::error::TrySendError::Closed(a)) => Err((self, a)),
            Err(mpsc::error::TrySendError::Full(a)) => Err((self, a)),
        }
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
    
    #[wasm_bindgen(js_name = "startWasm")]
    fn start_wasm(
        module: JsValue,
        memory: JsValue,
        ctx: JsValue,
        opts: WorkerOptions,
        builder: LoaderHelper,
        wasm_module: JsValue,
        wasm_memory: JsValue,
    ) -> Promise;
}

impl WebThreadPool {
    pub fn new(size: usize) -> Result<WebThreadPool, JsValue> {
        info!("pool::create(size={})", size);

        let (idle_tx1, idle_rx1) = mpsc::channel(MAX_MPSC);
        let (idle_tx3, idle_rx3) = mpsc::channel(MAX_MPSC);

        let (spawn_tx1, mut spawn_rx1) = mpsc::channel(MAX_MPSC);
        let (spawn_tx2, mut spawn_rx2) = mpsc::channel(MAX_MPSC);
        let (spawn_tx3, mut spawn_rx3) = mpsc::channel(MAX_MPSC);
        let (free_tx4, mut free_rx4) = mpsc::channel(MAX_MPSC);

        let pool_reactors = Arc::new(PoolState {
            idle_rx: Mutex::new(idle_rx1),
            idle_tx: idle_tx1,
            idx_seed: AtomicUsize::new(0),
            blocking: false,
            idle_size: 2usize.max(size),
            type_: PoolType::Shared,
            spawn: spawn_tx1,
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
        let pool3 = pool_dedicated.clone();

        // The management thread will spawn other threads - this thread is safe from
        // being blocked by other threads
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                select! {
                    spawn = spawn_rx1.recv() => {
                        if let Some(spawn) = spawn { pool1.expand(spawn); } else { break; }
                    }
                    spawn = spawn_rx2.recv() => {
                        if let Some(spawn) = spawn { let _ = _spawn_wasm(spawn).await; } else { break; }
                    }
                    spawn = spawn_rx3.recv() => {
                        if let Some(spawn) = spawn { pool3.expand(spawn); } else { break; }
                    }
                    free = free_rx4.recv() => {
                        if let Some(free) = free { _free_memory(free) } else { break; }
                    }
                }
            }
        });

        let pool = WebThreadPool {
            pool_reactors,
            pool_dedicated,
            spawn_wasm: Arc::new(spawn_tx2),
            free_memory: Arc::new(free_tx4),
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

    pub fn spawn_shared(
        &self,
        task: BoxRunAsync<'static, ()>
    ) {
        self.pool_reactors.spawn(Message::RunAsync(task));
    }
    
    pub fn spawn_wasm(
        &self,
        run: impl FnOnce(Store, Module, Option<VMMemory>) -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static,
        store: Store,
        mut module_bytes: Bytes,
        spawn_type: SpawnType,
    ) -> Result<(), WasiThreadError> {
        let run_type = match spawn_type {
            SpawnType::Create => WasmRunType::Create,
            SpawnType::CreateWithType(mem) => WasmRunType::CreateWithMemory(mem.ty),
            SpawnType::NewThread(_) => {
                let wasm_id = match THREAD_LOCAL_CURRENT_WASM.with(|c| c.borrow().clone()) {
                    Some(id) => id,
                    None => {
                        return Err(WasiThreadError::InvalidWasmContext);
                    }
                };
                WasmRunType::Existing(wasm_id)
            }
        };

        if module_bytes.starts_with(b"\0asm") == false {
            let parsed_bytes = wat::parse_bytes(module_bytes.as_ref()).map_err(|e| {
                error!("failed to parse WAT - {}", e);
                WasiThreadError::Unsupported
            })?;
            module_bytes = Bytes::from(parsed_bytes.to_vec());
        }        

        let msg = WasmRunCommand {
            run: Box::new(run),
            ty: run_type,
            store,
            module_bytes,
            free_memory: self.free_memory.clone(),
        };
        _spawn_send(&self.spawn_wasm, msg);
        Ok(())
    }

    pub fn spawn_dedicated(
        &self,
        task: BoxRun<'static>
    ) {
        self.pool_dedicated.spawn(Message::Run(task));
    }

    pub fn spawn_dedicated_async(
        &self,
        task: BoxRunAsync<'static, ()>
    ) {
        self.pool_dedicated.spawn(Message::RunAsync(task));
    }
}

async fn _compile_module(bytes: &[u8]) -> Result<js_sys::WebAssembly::Module, ()>
{
    let js_bytes = unsafe { Uint8Array::view(bytes) };
    Ok(
        match wasm_bindgen_futures::JsFuture::from(
            js_sys::WebAssembly::compile(&js_bytes.into())
        ).await {
            Ok(a) => match a.dyn_into::<js_sys::WebAssembly::Module>() {
                Ok(a) => a,
                Err(err) => {
                    error!("Failed to compile module - {}", err.as_string().unwrap_or_else(|| format!("{:?}", err)));
                    return Err(());
                }
            },
            Err(err) => {
                error!("WebAssembly failed to compile - {}", err.as_string().unwrap_or_else(|| format!("{:?}", err)));
                return Err(());
            }
        }
        //js_sys::WebAssembly::Module::new(&js_bytes.into()).unwrap()
    )
}

async fn _spawn_wasm(mut run: WasmRunCommand) -> Result<(),()> {
    let mut opts = WorkerOptions::new();
    opts.type_(WorkerType::Module);
    opts.name(&*format!("WasmWorker"));

    let result = match run.ty.clone() {
        WasmRunType::Create => {
            let wasm_module = _compile_module(&run.module_bytes[..]).await?;

            let wasm_id = WASM_SEED.fetch_add(1, Ordering::AcqRel);
            let ctx = WasmRunContext {
                id: wasm_id,
                cmd: run,
                memory: WasmRunMemory::WithoutMemory
            };
            let ctx = Box::into_raw(Box::new(ctx));

            wasm_bindgen_futures::JsFuture::from(start_wasm(
                wasm_bindgen::module(),
                wasm_bindgen::memory(),
                JsValue::from(ctx as u32),
                opts,
                LoaderHelper {},
                JsValue::from(wasm_module),
                JsValue::null(),
            ))
        }
        WasmRunType::CreateWithMemory(ty) => {
            if ty.shared == false {
                // We can only pass memory around between web workers when its a shared memory
                error!("Failed to create WASM process with external memory as only shared memory is supported yet this web assembly binary imports non-shared memory.");
                return Err(());
            }
            if ty.maximum.is_none() {
                // Browsers require maximum number defined on shared memory
                error!("Failed to create WASM process with external memory as shared memory must have a maximum size however this web assembly binary imports shared memory with no maximum defined.");
                return Err(());
            }

            let wasm_module = _compile_module(&run.module_bytes[..]).await?;

            let wasm_memory = {
                let descriptor = js_sys::Object::new();
                js_sys::Reflect::set(&descriptor, &"initial".into(), &ty.minimum.0.into()).unwrap();
                //let min = 100u32.max(ty.minimum.0);
                //js_sys::Reflect::set(&descriptor, &"initial".into(), &min.into()).unwrap();
                if let Some(max) = ty.maximum {
                    js_sys::Reflect::set(&descriptor, &"maximum".into(), &max.0.into()).unwrap();
                }
                js_sys::Reflect::set(&descriptor, &"shared".into(), &ty.shared.into()).unwrap();

                match js_sys::WebAssembly::Memory::new(&descriptor) {
                    Ok(a) => a,
                    Err(err) => {
                        error!("WebAssembly failed to create the memory - {}", err.as_string().unwrap_or_else(|| format!("{:?}", err)));
                        return Err(());
                    }
                }
            };

            let wasm_id = WASM_SEED.fetch_add(1, Ordering::AcqRel);
            THREAD_LOCAL_ROOT_WASM_INSTANCES.with(|c| {
                let mut root = c.borrow_mut();
                let root = root.deref_mut();
                root.insert(wasm_id, WasmInstance {
                    ref_cnt: 1,
                    module: wasm_module.clone(),
                    module_bytes: run.module_bytes.clone(),
                    memory: wasm_memory.clone(),
                    memory_type: ty.clone()
                })
            });

            let ctx = WasmRunContext {
                id: wasm_id,
                cmd: run,
                memory: WasmRunMemory::WithMemory(ty)
            };
            let ctx = Box::into_raw(Box::new(ctx));

            wasm_bindgen_futures::JsFuture::from(start_wasm(
                wasm_bindgen::module(),
                wasm_bindgen::memory(),
                JsValue::from(ctx as u32),
                opts,
                LoaderHelper {},
                JsValue::from(wasm_module),
                JsValue::from(wasm_memory),
            ))
        },
        WasmRunType::Existing(wasm_id) => {
            let inst = THREAD_LOCAL_ROOT_WASM_INSTANCES.with(|c| {
                let mut root = c.borrow_mut();
                let root = root.deref_mut();
                if let Some(inst) = root.get_mut(&wasm_id) {
                    inst.ref_cnt += 1;
                    Some(inst.clone())
                } else {
                    error!("WebAssembly Memory must be sent to the management thread before attempting to reuse it in a new WebWorker, it must also use SharedMemory");
                    None
                }
            });

            let wasm_module;
            let wasm_module_bytes;
            let wasm_memory;
            let wasm_memory_type;
            if let Some(inst) = inst {
                wasm_module = inst.module;
                wasm_module_bytes = inst.module_bytes;
                wasm_memory = inst.memory;
                wasm_memory_type = inst.memory_type;
            } else {
                return Err(());
            }
            run.module_bytes = wasm_module_bytes;
            
            let ctx = WasmRunContext {
                id: wasm_id,
                cmd: run,
                memory: WasmRunMemory::WithMemory(wasm_memory_type)
            };
            let ctx = Box::into_raw(Box::new(ctx));
        
            wasm_bindgen_futures::JsFuture::from(start_wasm(
                wasm_bindgen::module(),
                wasm_bindgen::memory(),
                JsValue::from(ctx as u32),
                opts,
                LoaderHelper {},
                JsValue::from(wasm_module),
                JsValue::from(wasm_memory),
            ))
        }
    };

    _process_worker_result(result, None).await;
    Ok(())
}

fn _free_memory(wasm_id: u32) {
    THREAD_LOCAL_ROOT_WASM_INSTANCES.with(|c| {
        let mut root = c.borrow_mut();
        let root = root.deref_mut();
        let should_remove = if let Some(mem) = root.get_mut(&wasm_id) {
            mem.ref_cnt -= 1;
            mem.ref_cnt <= 0
        } else {
            false
        };
        if should_remove {
            root.remove(&wasm_id);
        }
    })
}

fn _spawn_send<T: 'static>(tx: &mpsc::Sender<T>, mut msg: T) {
    for _ in 0..100 {
        match tx.try_send(msg) {
            Ok(_) => {
                return;
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                return;
            }
            Err(mpsc::error::TrySendError::Full(m)) => {
                msg = m;
            }
        }
        std::thread::yield_now();
    }

    if crate::common::is_worker() {
        let _ = tx.blocking_send(msg);
    } else {
        let spawn = tx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = spawn.send(msg).await;
        });
    }
}

impl PoolState {
    fn spawn(self: &Arc<Self>, mut msg: Message) {
        for _ in 0..10 {
            if let Ok(mut guard) = self.idle_rx.try_lock() {
                if let Ok(thread) = guard.try_recv() {
                    match thread.try_consume(msg) {
                        Ok(_) => {
                            return;
                        }
                        Err((thread, a)) => {
                            let _ = self.idle_tx.try_send(thread);
                            msg = a;
                        }
                    }
                }
                break;
            }
            std::thread::yield_now();
        }

        _spawn_send(&self.spawn, msg);   
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
            _process_worker_result(result, should_warn_on_error).await;
        });
    }
}

async fn _process_worker_result(result: JsFuture, should_warn_on_error: Option<Terminal>) {
    let ret = result.await;
    if let Err(err) = ret {
        let err = err
            .as_string()
            .unwrap_or_else(|| format!("{:?}", err));
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
        let work_tx = state.tx.clone();
        let pool = Arc::clone(&state.pool);
        let driver = async move {
            let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

            loop {
                // Process work until we need to go idle
                while let Some(task) = work {
                    match task {
                        Message::Run(task) => {
                            task();
                        }
                        Message::RunAsync(task) => {
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

#[wasm_bindgen(skip_typescript)]
pub fn wasm_entry_point(ctx_ptr: u32, wasm_module: JsValue, wasm_memory: JsValue)
{
    // Grab the run wrapper that passes us the rust variables (and extract the callback)
    let ctx = ctx_ptr as *mut WasmRunContext;
    let ctx = unsafe { Box::from_raw(ctx) };
    let run_callback = (*ctx).cmd.run;

    // Compile the web assembly module
    let wasm_store = ctx.cmd.store;
    let wasm_module = match wasm_module.dyn_into::<js_sys::WebAssembly::Module>() {
        Ok(a) => a,
        Err(err) => {
            error!("Failed to receive module - {}", err.as_string().unwrap_or_else(|| format!("{:?}", err)));
            _spawn_send(ctx.cmd.free_memory.deref(), ctx.id);
            return;
        }
    };
    let wasm_module = unsafe {
        match Module::from_js_module(&wasm_store, wasm_module, ctx.cmd.module_bytes.clone()){
            Ok(a) => a,
            Err(err) => {
                error!("Failed to compile module - {}", err);
                _spawn_send(ctx.cmd.free_memory.deref(), ctx.id);
                return;
            }
        }
    };

    // If memory was passed to the web worker then construct it
    let wasm_memory = match ctx.memory {
        WasmRunMemory::WithoutMemory => None,
        WasmRunMemory::WithMemory(wasm_memory_type) => {
            let wasm_memory = match wasm_memory.dyn_into::<js_sys::WebAssembly::Memory>() {
                Ok(a) => a,
                Err(err) => {
                    error!("Failed to receive memory for module - {}", err.as_string().unwrap_or_else(|| format!("{:?}", err)));
                    _spawn_send(ctx.cmd.free_memory.deref(), ctx.id);
                    return;
                }
            };
            Some(VMMemory::new(wasm_memory, wasm_memory_type))
        }
    };
    
    let name = js_sys::global()
        .unchecked_into::<DedicatedWorkerGlobalScope>()
        .name();
    debug!("{}: Entry", name);

    // Invoke the callback which will run the web assembly module
    let wasm_id = ctx.id;
    let free_memory = ctx.cmd.free_memory.clone();
    let driver = async move {
        THREAD_LOCAL_CURRENT_WASM.with(|c| c.borrow_mut().replace(wasm_id));
        run_callback(wasm_store, wasm_module, wasm_memory).await;
        THREAD_LOCAL_CURRENT_WASM.with(|c| c.borrow_mut().take());

        // Reduce the reference count on the memory
        _spawn_send(free_memory.deref(), wasm_id);
    };
    wasm_bindgen_futures::spawn_local(driver);
}
