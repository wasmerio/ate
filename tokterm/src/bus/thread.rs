use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use wasmer::Array;
use wasmer::ImportObject;
use wasmer::LazyInit;
use wasmer::Memory;
use wasmer::Module;
use wasmer::NativeFunc;
use wasmer::WasmPtr;
use wasmer::WasmerEnv;
use wasmer_wasi::WasiThread;

use super::*;

pub struct WasmBusThreadPool {
    threads: RwLock<HashMap<u32, WasmBusThread>>,
}

impl WasmBusThreadPool {
    pub fn new() -> Arc<WasmBusThreadPool> {
        Arc::new(WasmBusThreadPool {
            threads: RwLock::new(HashMap::default()),
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
        };

        let ret = WasmBusThread {
            thread_id: thread.thread_id(),
            pool: Arc::clone(self),
            factory: BusFactory::new(),
            inner: Arc::new(WasmBusThreadProtected {
                inside: RefCell::new(inner),
            }),
            memory: thread.memory_clone(),
            wasm_bus_free: LazyInit::new(),
            wasm_bus_malloc: LazyInit::new(),
            wasm_bus_data: LazyInit::new(),
            wasm_bus_error: LazyInit::new(),
        };

        threads.insert(thread_id, ret.clone());
        ret
    }
}

pub(super) struct WasmBusThreadInner {
    pub(super) invocations: HashMap<u32, Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
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
unsafe impl Sync for WasmBusThreadProtected {}

/// The environment provided to the WASI imports.
#[derive(Clone, WasmerEnv)]
pub struct WasmBusThread {
    pub(super) thread_id: u32,
    pool: Arc<WasmBusThreadPool>,
    pub(super) factory: BusFactory,
    pub(super) inner: Arc<WasmBusThreadProtected>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,

    #[wasmer(export(name = "wasm_bus_free"))]
    wasm_bus_free: LazyInit<NativeFunc<(WasmPtr<u8, Array>, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_malloc"))]
    wasm_bus_malloc: LazyInit<NativeFunc<u32, WasmPtr<u8, Array>>>,
    #[wasmer(export(name = "wasm_bus_data"))]
    wasm_bus_data: LazyInit<NativeFunc<(u32, WasmPtr<u8, Array>, u32), ()>>,
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
