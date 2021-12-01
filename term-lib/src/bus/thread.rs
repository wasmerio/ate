use crate::wasmer::Array;
use crate::wasmer::ImportObject;
use crate::wasmer::LazyInit;
use crate::wasmer::Memory;
use crate::wasmer::Module;
use crate::wasmer::NativeFunc;
use crate::wasmer::WasmPtr;
use crate::wasmer::WasmerEnv;
use crate::wasmer_wasi::WasiThread;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;

use super::*;

pub struct WasmBusThreadPool {
    threads: RwLock<HashMap<u32, WasmBusThread>>,
    process_factory: ProcessExecFactory,
}

impl WasmBusThreadPool {
    pub fn new(process_factory: ProcessExecFactory) -> Arc<WasmBusThreadPool> {
        Arc::new(WasmBusThreadPool {
            threads: RwLock::new(HashMap::default()),
            process_factory,
        })
    }

    pub fn get_or_create(self: &Arc<WasmBusThreadPool>, thread: &WasiThread) -> WasmBusThread {
        // fast path
        let thread_id = thread.thread_id();
        {
            let threads = self.threads.read().unwrap();
            if let Some(thread) = threads.get(&thread_id) {
                return thread.clone();
            }
        }

        // slow path
        let mut threads = self.threads.write().unwrap();
        if let Some(thread) = threads.get(&thread_id) {
            return thread.clone();
        }

        let inner = WasmBusThreadInner {
            invocations: HashMap::default(),
            factory: BusFactory::new(self.process_factory.clone()),
            callbacks: HashMap::default(),
            listens: HashSet::default(),
        };

        let ret = WasmBusThread {
            thread_id: thread.thread_id(),
            pool: Arc::clone(self),
            inner: Arc::new(WasmBusThreadProtected {
                inside: RefCell::new(inner),
            }),
            memory: thread.memory_clone(),
            wasm_bus_free: LazyInit::new(),
            wasm_bus_malloc: LazyInit::new(),
            wasm_bus_start: LazyInit::new(),
            wasm_bus_finish: LazyInit::new(),
            wasm_bus_error: LazyInit::new(),
        };

        threads.insert(thread_id, ret.clone());
        ret
    }
}

pub(super) struct WasmBusThreadInner {
    pub(super) invocations: HashMap<u32, Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
    pub(super) callbacks: HashMap<u32, HashMap<String, u32>>,
    pub(super) listens: HashSet<String>,
    pub(super) factory: BusFactory,
}

/// Caution! this class is used to access the protected area of the wasm bus thread
/// and makes no guantantees around accessing the insides concurrently. It is the
/// responsibility of the caller to ensure they do not call it concurrency.
pub(super) struct WasmBusThreadProtected {
    inside: RefCell<WasmBusThreadInner>,
}
impl WasmBusThreadProtected {
    pub(super) unsafe fn unwrap<'a>(&'a self) -> RefMut<'a, WasmBusThreadInner> {
        self.inside.borrow_mut()
    }
}
unsafe impl Send for WasmBusThreadProtected {}
unsafe impl Sync for WasmBusThreadProtected {}

/// The environment provided to the WASI imports.
#[derive(Clone, WasmerEnv)]
pub struct WasmBusThread {
    pub(super) thread_id: u32,
    pool: Arc<WasmBusThreadPool>,
    pub(super) inner: Arc<WasmBusThreadProtected>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,

    #[wasmer(export(name = "wasm_bus_free"))]
    wasm_bus_free: LazyInit<NativeFunc<(WasmPtr<u8, Array>, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_malloc"))]
    wasm_bus_malloc: LazyInit<NativeFunc<u32, WasmPtr<u8, Array>>>,
    #[wasmer(export(name = "wasm_bus_start"))]
    wasm_bus_start: LazyInit<NativeFunc<(u32, WasmPtr<u8, Array>, u32, WasmPtr<u8, Array>, u32), u32>>,
    #[wasmer(export(name = "wasm_bus_finish"))]
    wasm_bus_finish: LazyInit<NativeFunc<(u32, WasmPtr<u8, Array>, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_error"))]
    wasm_bus_error: LazyInit<NativeFunc<(u32, u32), ()>>,
}

impl WasmBusThread {
    /// Get an `ImportObject`
    pub fn import_object(&mut self, module: &Module) -> ImportObject {
        generate_import_object_wasm_bus(module.store(), self.clone())
    }

    /// Get a reference to the memory
    pub fn memory(&self) -> &Memory {
        self.memory_ref()
            .expect("Memory should be set on `WasiThread` first")
    }
}
