use crate::wasmer::Array;
use crate::wasmer::ImportObject;
use crate::wasmer::LazyInit;
use crate::wasmer::Memory;
use crate::wasmer::Module;
use crate::wasmer::NativeFunc;
use crate::wasmer::WasmPtr;
use crate::wasmer::WasmerEnv;
use crate::wasmer_wasi::WasiThread;
use async_trait::async_trait;
use serde::*;
use std::any::type_name;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::collections::HashSet;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::*;

use super::*;

use crate::api::*;
use crate::common::*;

pub struct WasmBusThreadPool {
    threads: RwLock<HashMap<u32, WasmBusThread>>,
    process_factory: ProcessExecFactory,
    work_register: Arc<RwLock<HashSet<CallHandle>>>,
}

impl WasmBusThreadPool {
    pub fn new(process_factory: ProcessExecFactory) -> Arc<WasmBusThreadPool> {
        Arc::new(WasmBusThreadPool {
            threads: RwLock::new(HashMap::default()),
            process_factory,
            work_register: Arc::new(RwLock::new(HashSet::default())),
        })
    }

    pub fn first(&self) -> Option<WasmBusThread> {
        let threads = self.threads.read().unwrap();
        threads
            .keys()
            .min()
            .map(|id| threads.get(id))
            .flatten()
            .map(|a| a.clone())
    }

    pub fn wake_all(&self) {
        let threads = self.threads.read().unwrap();
        for thread in threads.values() {
            let _ = thread.waker.wake();
        }
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

        let (work_tx, work_rx) = mpsc::channel(MAX_MPSC);
        let (polling_tx, polling_rx) = watch::channel(false);
        let inner = WasmBusThreadInner {
            invocations: HashMap::default(),
            calls: HashMap::default(),
            polling: polling_tx,
            factory: BusFactory::new(self.process_factory.clone()),
            callbacks: HashMap::default(),
            listens: HashSet::default(),
            work_rx,
        };

        let ret = WasmBusThread {
            waker: Arc::new(ThreadWaker::new(work_tx.clone(), polling_rx.clone())),
            thread_id: thread.thread_id(),
            system: System::default(),
            pool: Arc::clone(self),
            polling: polling_rx,
            inner: Arc::new(WasmBusThreadProtected {
                inside: RefCell::new(inner),
            }),
            work_tx,
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

#[derive(Debug, Clone)]
pub struct WasmBusThreadHandle {
    pub handle: CallHandle,
    pub work_register: Arc<RwLock<HashSet<CallHandle>>>,
}

impl WasmBusThreadHandle {
    pub fn new(
        handle: CallHandle,
        work_register: &Arc<RwLock<HashSet<CallHandle>>>,
    ) -> WasmBusThreadHandle {
        WasmBusThreadHandle {
            handle,
            work_register: Arc::clone(work_register),
        }
    }

    pub fn handle(&self) -> CallHandle {
        self.handle
    }
}

impl Drop for WasmBusThreadHandle {
    fn drop(&mut self) {
        let mut work_register = self.work_register.write().unwrap();
        work_register.remove(&self.handle);
    }
}

#[derive(Debug, Clone)]
pub(crate) enum WasmBusThreadWork {
    Wake,
    Call {
        topic: String,
        parent: Option<CallHandle>,
        handle: WasmBusThreadHandle,
        data: Vec<u8>,
        tx: mpsc::Sender<Result<Vec<u8>, CallError>>,
    },
}

pub(super) struct WasmBusThreadInner {
    pub(super) invocations: HashMap<u32, Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
    pub(super) calls: HashMap<u32, mpsc::Sender<Result<Vec<u8>, CallError>>>,
    pub(super) polling: watch::Sender<bool>,
    pub(super) callbacks: HashMap<u32, HashMap<String, u32>>,
    pub(super) listens: HashSet<String>,
    pub(super) factory: BusFactory,
    #[allow(dead_code)]
    pub(crate) work_rx: mpsc::Receiver<WasmBusThreadWork>,
}

/// Caution! this class is used to access the protected area of the wasm bus thread
/// and makes no guantantees around accessing the insides concurrently. It is the
/// responsibility of the caller to ensure they do not call it concurrency.
pub(crate) struct WasmBusThreadProtected {
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
    pub(crate) system: System,
    pub thread_id: u32,
    pub(crate) waker: Arc<ThreadWaker>,
    pub pool: Arc<WasmBusThreadPool>,
    pub polling: watch::Receiver<bool>,
    pub(crate) inner: Arc<WasmBusThreadProtected>,
    pub(crate) work_tx: mpsc::Sender<WasmBusThreadWork>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,

    #[wasmer(export(name = "wasm_bus_free"))]
    wasm_bus_free: LazyInit<NativeFunc<(WasmPtr<u8, Array>, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_malloc"))]
    wasm_bus_malloc: LazyInit<NativeFunc<u32, WasmPtr<u8, Array>>>,
    #[wasmer(export(name = "wasm_bus_start"))]
    wasm_bus_start:
        LazyInit<NativeFunc<(u32, u32, WasmPtr<u8, Array>, u32, WasmPtr<u8, Array>, u32), ()>>,
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

    fn generate_handle(&self) -> WasmBusThreadHandle {
        let mut work_register = self.pool.work_register.write().unwrap();
        loop {
            let handle: CallHandle = fastrand::u32(..).into();
            if work_register.contains(&handle) == false {
                work_register.insert(handle);
                drop(work_register);
                return WasmBusThreadHandle::new(handle, &self.pool.work_register);
            }
        }
    }

    /// Issues work on the BUS
    fn call_internal(
        &self,
        parent: Option<CallHandle>,
        topic: String,
        data: Vec<u8>,
    ) -> (
        mpsc::Receiver<Result<Vec<u8>, CallError>>,
        WasmBusThreadHandle,
    ) {
        // Create a call handle
        let handle = self.generate_handle();

        // Build the call that will be sent
        let (tx, rx) = mpsc::channel(1);
        let mut msg = WasmBusThreadWork::Call {
            topic,
            parent,
            handle: handle.clone(),
            data,
            tx,
        };

        // If we are already polling then try and send it instantly
        if *self.polling.borrow() == true {
            match self.work_tx.try_send(msg) {
                Ok(_) => {
                    return (rx, handle);
                },
                Err(mpsc::error::TrySendError::Closed(a)) => {
                    msg = a;
                },
                Err(mpsc::error::TrySendError::Full(a)) => {
                    msg = a;
                }
            }
        }

        // Otherwise we need to do it asynchronously
        let work_tx = self.work_tx.clone();
        let polling = self.polling.clone();
        self.system.fork_shared(move || async move {
            if async_wait_for_poll(polling).await {
                let _ = work_tx.send(msg).await;
            }
        });

        // Return the receiver
        (rx, handle)
    }

    /// Issues work on the BUS
    pub fn call_raw(
        &self,
        parent: Option<CallHandle>,
        topic: String,
        data: Vec<u8>,
    ) -> AsyncWasmBusResultRaw {
        let (rx, handle) = self.call_internal(parent, topic, data);
        AsyncWasmBusResultRaw::new(rx, handle)
    }

    pub fn call<RES, REQ>(&self, request: REQ) -> Result<AsyncWasmBusResult<RES>, CallError>
    where
        REQ: Serialize,
        RES: de::DeserializeOwned,
    {
        // Serialize
        let topic = type_name::<REQ>();
        let data = match bincode::serialize(&request) {
            Ok(a) => a,
            Err(_err) => {
                return Err(CallError::SerializationFailed);
            }
        };

        let (rx, handle) = self.call_internal(None, topic.to_string(), data);
        Ok(AsyncWasmBusResult::new(self, rx, handle))
    }

    pub fn wait_for_poll(&self) -> bool {
        // fast path
        if *self.polling.borrow() == false {
            // slow path
            let mut polling = self.polling.clone();
            if let None = self
                .system
                .spawn_dedicated(move || async move {
                    while *polling.borrow() == false {
                        if let Err(_) = polling.changed().await {
                            return;
                        }
                    }
                })
                .block_on()
            {
                return false;
            }
        }

        return true;
    }

    pub async fn async_wait_for_poll(&mut self) -> bool {
        async_wait_for_poll(self.polling.clone()).await
    }
}

async fn async_wait_for_poll(mut polling: watch::Receiver<bool>) -> bool {
    while *polling.borrow() == false {
        if let Err(_) = polling.changed().await {
            return false;
        }
    }
    return true;
}

pub struct AsyncWasmBusResultRaw {
    pub(crate) rx: mpsc::Receiver<Result<Vec<u8>, CallError>>,
    pub(crate) handle: WasmBusThreadHandle,
}

impl AsyncWasmBusResultRaw {
    pub fn new(
        rx: mpsc::Receiver<Result<Vec<u8>, CallError>>,
        handle: WasmBusThreadHandle,
    ) -> Self {
        Self { rx, handle }
    }

    pub fn handle(&self) -> WasmBusThreadHandle {
        self.handle.clone()
    }

    pub fn block_on(mut self) -> Result<Vec<u8>, CallError> {
        self.rx.blocking_recv().ok_or_else(|| CallError::Aborted)?
    }

    pub async fn join(mut self) -> Result<Vec<u8>, CallError> {
        self.rx.recv().await.ok_or_else(|| CallError::Aborted)?
    }
}

#[async_trait]
impl Invokable for AsyncWasmBusResultRaw {
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        self.rx.recv().await.ok_or_else(|| CallError::Aborted)?
    }
}

pub struct AsyncWasmBusResult<T>
where
    T: de::DeserializeOwned,
{
    pub(crate) thread: WasmBusThread,
    pub(crate) handle: WasmBusThreadHandle,
    pub(crate) rx: mpsc::Receiver<Result<Vec<u8>, CallError>>,
    _marker: PhantomData<T>,
}

impl<T> AsyncWasmBusResult<T>
where
    T: de::DeserializeOwned,
{
    pub fn new(
        thread: &WasmBusThread,
        rx: mpsc::Receiver<Result<Vec<u8>, CallError>>,
        handle: WasmBusThreadHandle,
    ) -> Self {
        Self {
            thread: thread.clone(),
            handle,
            rx,
            _marker: PhantomData,
        }
    }

    pub fn block_on(mut self) -> Result<T, CallError> {
        let data = self
            .rx
            .blocking_recv()
            .ok_or_else(|| CallError::Aborted)??;
        match bincode::deserialize::<T>(&data[..]) {
            Ok(a) => Ok(a),
            Err(_err) => Err(CallError::SerializationFailed),
        }
    }

    pub async fn join(mut self) -> Result<T, CallError> {
        let data = self.rx.recv().await.ok_or_else(|| CallError::Aborted)??;
        match bincode::deserialize::<T>(&data[..]) {
            Ok(a) => Ok(a),
            Err(_err) => Err(CallError::SerializationFailed),
        }
    }

    pub fn call<RES, REQ>(&self, request: REQ) -> Result<AsyncWasmBusResult<RES>, CallError>
    where
        REQ: Serialize,
        RES: de::DeserializeOwned,
    {
        // Serialize
        let topic = type_name::<REQ>();
        let data = match bincode::serialize(&request) {
            Ok(a) => a,
            Err(_err) => {
                return Err(CallError::SerializationFailed);
            }
        };

        let (rx, handle) =
            self.thread
                .call_internal(Some(self.handle.handle()), topic.to_string(), data);
        Ok(AsyncWasmBusResult::new(&self.thread, rx, handle))
    }

    pub fn session(self) -> AsyncWasmBusSession {
        AsyncWasmBusSession {
            thread: self.thread,
            handle: self.handle,
        }
    }
}

#[derive(Clone)]
pub struct AsyncWasmBusSession {
    pub(crate) thread: WasmBusThread,
    pub(crate) handle: WasmBusThreadHandle,
}

impl AsyncWasmBusSession {
    pub fn new(thread: &WasmBusThread, handle: WasmBusThreadHandle) -> Self {
        Self {
            thread: thread.clone(),
            handle,
        }
    }

    pub fn call<RES, REQ>(&self, request: REQ) -> Result<AsyncWasmBusResult<RES>, CallError>
    where
        REQ: Serialize,
        RES: de::DeserializeOwned,
    {
        // Serialize
        let topic = type_name::<REQ>();
        let data = match bincode::serialize(&request) {
            Ok(a) => a,
            Err(_err) => {
                return Err(CallError::SerializationFailed);
            }
        };

        let (rx, handle) =
            self.thread
                .call_internal(Some(self.handle.handle()), topic.to_string(), data);
        Ok(AsyncWasmBusResult::new(&self.thread, rx, handle))
    }
}
