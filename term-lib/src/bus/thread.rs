use crate::wasmer::ImportObject;
use crate::wasmer::LazyInit;
use crate::wasmer::Memory;
use crate::wasmer::Module;
use crate::wasmer::NativeFunc;
use crate::wasmer::WasmerEnv;
use crate::wasmer_wasi::WasiError;
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
use std::task::Context;
use std::task::Poll;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::*;

use super::*;

use crate::eval::EvalContext;
use crate::api::*;
use crate::err;

pub struct WasmBusThreadPool {
    threads: Arc<RwLock<HashMap<u32, WasmBusThread>>>,
    process_factory: ProcessExecFactory,
    ctx: WasmCallerContext,
}

impl WasmBusThreadPool {
    pub fn new(process_factory: ProcessExecFactory, ctx: WasmCallerContext) -> Arc<WasmBusThreadPool> {
        Arc::new(WasmBusThreadPool {
            threads: Arc::new(RwLock::new(HashMap::default())),
            process_factory,
            ctx,
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

        let (work_tx, work_rx) = mpsc::channel(crate::common::MAX_MPSC);
        let (polling_tx, polling_rx) = watch::channel(false);
        let (feed_tx, feed_rx) = mpsc::channel(crate::common::MAX_MPSC);

        let inner = WasmBusThreadInner {
            invocations: HashMap::default(),
            feed_data: feed_rx,
            calls: HashMap::default(),
            factory: BusFactory::new(self.process_factory.clone()),
            callbacks: HashMap::default(),
            listens: HashSet::default(),
            polling: polling_tx,
            work_rx: Some(work_rx),
            poll_thread: None,
        };

        let ret = WasmBusThread {
            thread_id: thread.thread_id(),
            system: System::default(),
            pool: Arc::clone(self),
            polling: polling_rx,
            inner: Arc::new(WasmBusThreadProtected {
                inside: RefCell::new(inner),
            }),
            work_tx,
            feed_data: feed_tx,
            ctx: self.ctx.clone(),
            memory: thread.memory_clone(),
            wasm_bus_free: LazyInit::new(),
            wasm_bus_malloc: LazyInit::new(),
            wasm_bus_start: LazyInit::new(),
            wasm_bus_finish: LazyInit::new(),
            wasm_bus_error: LazyInit::new(),
            wasm_bus_drop: LazyInit::new(),
        };

        threads.insert(thread_id, ret.clone());
        ret
    }

    pub fn take_context(&self) -> Option<EvalContext> {
        self.process_factory.take_context()
    }
}

#[derive(Debug, Clone)]
pub struct WasmBusThreadHandle {
    pub handle: CallHandle,
}

impl WasmBusThreadHandle {
    pub fn new(handle: CallHandle) -> WasmBusThreadHandle {
        WasmBusThreadHandle { handle }
    }

    pub fn handle(&self) -> CallHandle {
        self.handle
    }
}

#[derive(Debug, Clone)]
pub(crate) enum WasmBusThreadWork {
    Call {
        topic: String,
        parent: Option<CallHandle>,
        handle: WasmBusThreadHandle,
        data: Vec<u8>,
        tx: mpsc::Sender<Result<Vec<u8>, CallError>>,
    },
    Drop {
        handle: CallHandle,
    },
}

pub(crate) struct WasmBusThreadInvocation {
    pub _abort: mpsc::Sender<()>,
    pub result: AsyncResult<Result<InvokeResult, CallError>>,
    pub data_feeder: WasmBusCallback,
}

pub(crate) struct WasmBusThreadInner {
    pub(super) invocations: HashMap<CallHandle, WasmBusThreadInvocation>,
    pub(super) feed_data: mpsc::Receiver<FeedData>,
    pub(super) calls: HashMap<CallHandle, mpsc::Sender<Result<Vec<u8>, CallError>>>,
    pub(super) callbacks: HashMap<CallHandle, HashMap<String, CallHandle>>,
    pub(super) listens: HashSet<String>,
    pub(super) factory: BusFactory,
    #[allow(dead_code)]
    pub(crate) polling: watch::Sender<bool>,
    #[allow(dead_code)]
    pub(crate) work_rx: Option<mpsc::Receiver<WasmBusThreadWork>>,
    #[allow(dead_code)]
    pub(crate) poll_thread: Option<Pin<Box<dyn Future<Output = u32> + Send + 'static>>>,
}

/// Caution! this class is used to access the protected area of the wasm bus thread
/// and makes no guantantees around accessing the insides concurrently. It is the
/// responsibility of the caller to ensure they do not call it concurrency.
pub(crate) struct WasmBusThreadProtected {
    inside: RefCell<WasmBusThreadInner>,
}
impl WasmBusThreadProtected {
    pub(crate) unsafe fn lock<'a>(&'a self) -> RefMut<'a, WasmBusThreadInner> {
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
    pub pool: Arc<WasmBusThreadPool>,
    pub polling: watch::Receiver<bool>,
    pub(crate) inner: Arc<WasmBusThreadProtected>,
    pub(crate) work_tx: mpsc::Sender<WasmBusThreadWork>,
    pub(super) feed_data: mpsc::Sender<FeedData>,
    pub(crate) ctx: WasmCallerContext,

    #[wasmer(export)]
    pub memory: LazyInit<Memory>,
    #[wasmer(export(name = "wasm_bus_free"))]
    pub wasm_bus_free: LazyInit<NativeFunc<(u32, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_malloc"))]
    pub wasm_bus_malloc: LazyInit<NativeFunc<u32, u32>>,
    #[wasmer(export(name = "wasm_bus_start"))]
    pub wasm_bus_start: LazyInit<NativeFunc<(u32, u32, u32, u32, u32, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_finish"))]
    pub wasm_bus_finish: LazyInit<NativeFunc<(u32, u32, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_error"))]
    pub wasm_bus_error: LazyInit<NativeFunc<(u32, u32), ()>>,
    #[wasmer(export(name = "wasm_bus_drop"))]
    pub wasm_bus_drop: LazyInit<NativeFunc<u32, ()>>,
}

impl Future
for WasmBusThread
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
    {
        let sessions;
        let mut to_remove = Vec::new();
        let mut callbacks = Vec::new();
        unsafe {
            let mut inner = self.inner.lock();
            for (handle, invocation) in inner.invocations.iter_mut() {
                let mut rx = Pin::new(&mut invocation.result.rx);
                match rx.poll_recv(cx) {
                    Poll::Ready(Some(result)) => {
                        callbacks.push((invocation.data_feeder.clone(), result));
                        to_remove.push(handle.clone());
                    }
                    Poll::Ready(None) => {
                        callbacks.push((invocation.data_feeder.clone(), Err(CallError::Aborted)));
                        to_remove.push(handle.clone());
                    }
                    Poll::Pending => {
                        continue;
                    }
                }
            }
            sessions = inner.factory.sessions();
        }

        for (callback, result) in callbacks {
            callback.process(result, &sessions);
        }

        let mut feed_data = Vec::new();
        unsafe {
            let mut inner = self.inner.lock();
            while let Poll::Ready(Some(result)) = inner.feed_data.poll_recv(cx) {
                feed_data.push(result);
            }
        }

        self.feed_data(feed_data);

        let mut delayed_drop = Vec::new();
        unsafe {
            let mut inner = self.inner.lock();
            for handle in to_remove {
                delayed_drop.push(inner.invocations.remove(&handle));
            }
        }
        drop(delayed_drop);

        Poll::Pending
    }
}

impl WasmBusThread
{
    pub fn process(&self) -> usize
    {
        let sessions;
        let mut to_remove = Vec::new();
        let mut callbacks = Vec::new();
        unsafe {
            let mut inner = self.inner.lock();
            for (handle, invocation) in inner.invocations.iter_mut() {
                match invocation.result.rx.try_recv() {
                    Ok(result) => {
                        callbacks.push((invocation.data_feeder.clone(), result));
                        to_remove.push(handle.clone());
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        callbacks.push((invocation.data_feeder.clone(), Err(CallError::Aborted)));
                        to_remove.push(handle.clone());
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {
                        continue;
                    }
                }
            }
            sessions = inner.factory.sessions();
        }

        let mut ret = 0usize;
        for (callback, result) in callbacks {
            callback.process(result, &sessions);
            ret += 1;
        }

        let mut feed_data = Vec::new();
        unsafe {
            let mut inner = self.inner.lock();
            while let Ok(result) = inner.feed_data.try_recv() {
                feed_data.push(result);
            }
        }

        ret += feed_data.len();
        self.feed_data(feed_data);

        let mut delayed_drop = Vec::new();
        unsafe {
            let mut inner = self.inner.lock();
            for handle in to_remove {
                delayed_drop.push(inner.invocations.remove(&handle));
            }
        }
        drop(delayed_drop);

        ret
    }

    pub fn feed_data(&self, feeds: Vec<FeedData>)
    {
        if feeds.len() <= 0 {
            return;
        }

        let native_memory = self.memory_ref();
        let native_malloc = self.wasm_bus_malloc_ref();
        let native_finish = self.wasm_bus_finish_ref();
        let native_error = self.wasm_bus_error_ref();
        if native_memory.is_none() || native_malloc.is_none() || native_finish.is_none() || native_error.is_none() {
            warn!("wasm-bus::call - ABI does not match (finish)");
            return;
        }
        let native_memory = native_memory.unwrap();
        let native_malloc = native_malloc.unwrap();
        let native_finish = native_finish.unwrap();
        let native_error = native_error.unwrap();

        for feed in feeds {
            match feed {
                FeedData::Finish { handle, data } => {
                    trace!(
                        "wasm-bus::call-reply (handle={}, response={} bytes)",
                        handle.id,
                        data.len()
                    );
                    let buf_len = data.len() as u32;
                    let buf = native_malloc.call(buf_len).unwrap();

                    native_memory
                        .uint8view_with_byte_offset_and_length(buf, buf_len)
                        .copy_from(&data[..]);

                    native_finish
                        .call(handle.id, buf, buf_len)
                        .unwrap();
                }
                FeedData::Error { handle, err } => {
                    trace!(
                        "wasm-bus::call-reply (handle={}, error={})",
                        handle.id,
                        err
                    );
                    native_error.call(handle.id, err.into()).unwrap();
                }
            }
        }
    }
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
        let handle: CallHandle = fastrand::u32(..).into();
        return WasmBusThreadHandle::new(handle);
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

        // Build the call and send it
        let (tx, rx) = mpsc::channel(1);
        self.send_internal(WasmBusThreadWork::Call {
            topic,
            parent,
            handle: handle.clone(),
            data,
            tx,
        });
        (rx, handle)
    }

    fn send_internal(&self, msg: WasmBusThreadWork) {
        self.system.fork_send(&self.work_tx, msg);
    }

    /// Issues work on the BUS
    pub fn call_raw(
        &self,
        parent: Option<CallHandle>,
        topic: String,
        data: Vec<u8>,
        ctx: WasmCallerContext,
    ) -> AsyncWasmBusResultRaw {
        let (rx, handle) = self.call_internal(parent, topic, data);
        AsyncWasmBusResultRaw::new(rx, handle, ctx, self.ctx.clone())
    }

    pub fn call<RES, REQ>(
        &self,
        format: SerializationFormat,
        request: REQ,
        ctx: WasmCallerContext,
    ) -> Result<AsyncWasmBusResult<RES>, CallError>
    where
        REQ: Serialize,
        RES: de::DeserializeOwned,
    {
        // Serialize
        let topic = type_name::<REQ>();
        let data = match format {
            SerializationFormat::Bincode => match bincode::serialize(&request) {
                Ok(a) => a,
                Err(err) => {
                    debug!(
                        "failed to serialize the request object (type={}, format={}) - {}",
                        type_name::<REQ>(),
                        format,
                        err
                    );
                    return Err(CallError::SerializationFailed);
                }
            },
            SerializationFormat::Json => match serde_json::to_vec(&request) {
                Ok(a) => a,
                Err(err) => {
                    debug!(
                        "failed to serialize the request object (type={}, format={}) - {}",
                        type_name::<REQ>(),
                        format,
                        err
                    );
                    return Err(CallError::SerializationFailed);
                }
            },
        };

        let (rx, handle) = self.call_internal(None, topic.to_string(), data);
        Ok(AsyncWasmBusResult::new(self, rx, handle, format, ctx))
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

    pub(crate) async unsafe fn work(&self, work: WasmBusThreadWork) -> u32 {
        // Upon receiving some work we will process it
        match work {
            WasmBusThreadWork::Call {
                topic,
                parent,
                handle,
                data,
                tx,
            } => {
                let native_memory = self.memory_ref();
                let native_malloc = self.wasm_bus_malloc_ref();
                let native_start = self.wasm_bus_start_ref();
                if native_memory.is_none() || native_malloc.is_none() || native_start.is_none() {
                    let _ = tx.send(Err(CallError::IncorrectAbi));
                    warn!("wasm-bus::call - ABI does not match (start)");
                    return err::ERR_PANIC;
                }
                let native_memory = native_memory.unwrap();
                let native_malloc = native_malloc.unwrap();
                let native_start = native_start.unwrap();

                // Check the listening is of the correct type
                let no_topic = {
                    let inner = self.inner.lock();
                    inner.listens.contains(&topic)
                };
                if no_topic == false {
                    let _ = tx.send(Err(CallError::InvalidTopic));
                    warn!("wasm-bus::call - invalid topic");
                    return err::ERR_OK;
                }

                // Determine the parent handle
                let parent = parent.map(|a| a.into()).unwrap_or(u32::MAX);

                // Invoke the call
                let topic_bytes = topic.as_bytes();
                let topic_len = topic_bytes.len() as u32;
                let topic_ptr = match native_malloc.call(topic_len) {
                    Ok(a) => a,
                    Err(err) => {
                        warn!(
                            "wasm-bus::call - allocation failed (topic={}, len={}) - {} - {}",
                            topic, topic_len, err, err.message()
                        );
                        let _ = tx.send(Err(CallError::MemoryAllocationFailed));
                        return err::ERR_OK;
                    }
                };
                native_memory
                    .uint8view_with_byte_offset_and_length(topic_ptr, topic_len)
                    .copy_from(&topic_bytes[..]);

                let request_bytes = &data[..];
                let request_len = request_bytes.len() as u32;
                let request_ptr = match native_malloc.call(request_len) {
                    Ok(a) => a,
                    Err(err) => {
                        warn!(
                            "wasm-bus::call - allocation failed (topic={}, len={}) - {} - {}",
                            topic, request_len, err, err.message()
                        );
                        let _ = tx.send(Err(CallError::MemoryAllocationFailed));
                        return err::ERR_OK;
                    }
                };
                native_memory
                    .uint8view_with_byte_offset_and_length(request_ptr, request_len)
                    .copy_from(&request_bytes[..]);

                // Record the handler so that when the call completes it notifies the
                // one who put this work on the queue
                let handle = handle.handle();
                {
                    let mut inner = self.inner.lock();
                    inner.calls.insert(handle, tx);
                }

                // Attempt to make the call to the WAPM module
                match native_start.call(
                    parent,
                    handle.id,
                    topic_ptr,
                    topic_len,
                    request_ptr,
                    request_len,
                ) {
                    Ok(_) => err::ERR_OK,
                    Err(e) => {
                        warn!(
                            "wasm-bus::call - invocation failed (topic={}) - {} - {}",
                            topic, e, e.message()
                        );
                        let call = {
                            let mut inner = self.inner.lock();
                            inner.calls.remove(&handle)
                        };
                        if let Some(call) = call {
                            let _ = call.send(Err(CallError::BusInvocationFailed));
                        }
                        match e.downcast::<WasiError>() {
                            Ok(WasiError::Exit(code)) => code,
                            Ok(WasiError::UnknownWasiVersion) => crate::err::ERR_PANIC,
                            Err(_) => err::ERR_PANIC,
                        }
                    }
                }
            }
            WasmBusThreadWork::Drop { handle } => {
                if let Some(native_drop) = self.wasm_bus_drop_ref() {
                    if let Err(err) = native_drop.call(handle.id) {
                        warn!("wasm-bus::drop - runtime error - {} - {}", err, err.message());
                    }
                }
                super::syscalls::wasm_bus_drop(self, handle);
                err::ERR_OK
            }
        }
    }

    pub async fn async_wait_for_poll(&self) -> bool {
        async_wait_for_poll(self.polling.clone()).await
    }

    pub fn drop_call(&self, handle: CallHandle) {
        self.send_internal(WasmBusThreadWork::Drop { handle });
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
    pub(crate) ctx_src: WasmCallerContext,
    pub(crate) ctx_dst: WasmCallerContext,
}

impl AsyncWasmBusResultRaw {
    pub fn new(
        rx: mpsc::Receiver<Result<Vec<u8>, CallError>>,
        handle: WasmBusThreadHandle,
        ctx_src: WasmCallerContext,
        ctx_dst: WasmCallerContext,
    ) -> Self {
        Self {
            rx,
            handle,
            ctx_src,
            ctx_dst,
        }
    }

    pub fn handle(&self) -> WasmBusThreadHandle {
        self.handle.clone()
    }

    pub fn block_on(mut self) -> Result<Vec<u8>, CallError> {
        let mut tick_wait = 0u64;
        loop {
            // Attempt to get the data from the receiver pipe
            match self.rx.try_recv() {
                Ok(msg) => {
                    return msg;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(CallError::Aborted);
                }
            }

            // Check for a forced exit
            if self.ctx_src.should_terminate().is_some() {
                return Err(CallError::Aborted);
            }
            if self.ctx_dst.should_terminate().is_some() {
                return Err(CallError::Aborted);
            }
            
            // Linearly increasing wait time
            tick_wait += 1;
            let wait_time = u64::min(tick_wait / 10, 20);
            std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
        }
    }
}

#[async_trait]
impl Invokable for AsyncWasmBusResultRaw {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| CallError::Aborted)?
            .map(|a| InvokeResult::Response(a))
    }
}

pub struct AsyncWasmBusResult<T>
where
    T: de::DeserializeOwned,
{
    pub(crate) thread: WasmBusThread,
    pub(crate) handle: WasmBusThreadHandle,
    pub(crate) format: SerializationFormat,
    pub(crate) rx: mpsc::Receiver<Result<Vec<u8>, CallError>>,
    pub(crate) ctx: WasmCallerContext,
    should_drop: bool,
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
        format: SerializationFormat,
        ctx: WasmCallerContext,
    ) -> Self {
        Self {
            thread: thread.clone(),
            handle,
            format,
            rx,
            should_drop: true,
            ctx,
            _marker: PhantomData,
        }
    }

    pub fn block_on(mut self) -> Result<T, CallError> {
        self.block_on_internal()
    }

    fn block_on_internal(&mut self) -> Result<T, CallError> {
        let format = self.format;
        let mut tick_wait = 0u64;
        loop {
            // Attempt to get the data from the receiver pipe
            match self.rx.try_recv() {
                Ok(msg) => {
                    let data = msg?;
                    self.should_drop = false;
                    return Self::process_block_on_result(format, data);
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(CallError::Aborted);
                }
            }

            // Check for a forced exit
            if self.ctx.should_terminate().is_some() {
                return Err(CallError::Aborted);
            }
            if self.thread.ctx.should_terminate().is_some() {
                return Err(CallError::Aborted);
            }
            
            // Linearly increasing wait time
            tick_wait += 1;
            let wait_time = u64::min(tick_wait / 10, 20);
            std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
        }
    }

    fn process_block_on_result(format: SerializationFormat, data: Vec<u8>) -> Result<T, CallError> {
        match format {
            SerializationFormat::Bincode => match bincode::deserialize::<T>(&data[..]) {
                Ok(a) => Ok(a),
                Err(err) => {
                    debug!(
                        "failed to deserialize the response object (type={}, format={}) - {}",
                        type_name::<T>(),
                        format,
                        err
                    );
                    Err(CallError::SerializationFailed)
                }
            },
            SerializationFormat::Json => match serde_json::from_slice::<T>(&data[..]) {
                Ok(a) => Ok(a),
                Err(err) => {
                    debug!(
                        "failed to deserialize the response object (type={}, format={}) - {}",
                        type_name::<T>(),
                        format,
                        err
                    );
                    Err(CallError::SerializationFailed)
                }
            },
        }
    }

    pub async fn join(mut self) -> Result<T, CallError> {
        self.join_internal().await
    }

    async fn join_internal(&mut self) -> Result<T, CallError> {
        let data = self.rx.recv().await.ok_or_else(|| CallError::Aborted)??;
        self.should_drop = false;
        match self.format {
            SerializationFormat::Bincode => match bincode::deserialize::<T>(&data[..]) {
                Ok(a) => Ok(a),
                Err(err) => {
                    debug!(
                        "failed to deserialize the response object (type={}, format={}) - {}",
                        type_name::<T>(),
                        self.format,
                        err
                    );
                    Err(CallError::SerializationFailed)
                }
            },
            SerializationFormat::Json => match serde_json::from_slice::<T>(&data[..]) {
                Ok(a) => Ok(a),
                Err(err) => {
                    debug!(
                        "failed to deserialize the response object (type={}, format={}) - {}",
                        type_name::<T>(),
                        self.format,
                        err
                    );
                    Err(CallError::SerializationFailed)
                }
            },
        }
    }

    pub async fn detach(mut self) -> Result<AsyncWasmBusSession, CallError> {
        self.should_drop = false;
        let _ = self.join_internal().await?;
        Ok(AsyncWasmBusSession::new(
            &self.thread,
            self.handle.clone(),
            self.format.clone(),
        ))
    }

    pub fn blocking_detach(mut self) -> Result<AsyncWasmBusSession, CallError> {
        self.should_drop = false;
        let _ = self.block_on_internal()?;
        Ok(AsyncWasmBusSession::new(
            &self.thread,
            self.handle.clone(),
            self.format.clone(),
        ))
    }
}

impl<T> Drop for AsyncWasmBusResult<T>
where
    T: de::DeserializeOwned,
{
    fn drop(&mut self) {
        if self.should_drop == true {
            self.thread.drop_call(self.handle.handle());
        }
    }
}

pub struct WasmBusSessionMarker {
    thread: WasmBusThread,
    handle: CallHandle,
}

impl Drop for WasmBusSessionMarker {
    fn drop(&mut self) {
        debug!("bus sesssion closed - handle={}", self.handle);
        self.thread.drop_call(self.handle);
    }
}

#[derive(Clone)]
pub struct AsyncWasmBusSession {
    pub(crate) handle: WasmBusThreadHandle,
    pub(crate) format: SerializationFormat,
    pub(crate) marker: Arc<WasmBusSessionMarker>,
}

impl AsyncWasmBusSession {
    pub fn new(
        thread: &WasmBusThread,
        handle: WasmBusThreadHandle,
        format: SerializationFormat,
    ) -> Self {
        Self {
            marker: Arc::new(WasmBusSessionMarker {
                thread: thread.clone(),
                handle: handle.handle(),
            }),
            handle,
            format,
        }
    }

    pub fn id(&self) -> CallHandle {
        self.handle.handle
    }

    pub fn call<RES, REQ>(&self, request: REQ, ctx: WasmCallerContext) -> Result<AsyncWasmBusResult<RES>, CallError>
    where
        REQ: Serialize,
        RES: de::DeserializeOwned,
    {
        self.call_with_format(self.format.clone(), request, ctx)
    }

    pub fn call_with_format<RES, REQ>(
        &self,
        format: SerializationFormat,
        request: REQ,
        ctx: WasmCallerContext
    ) -> Result<AsyncWasmBusResult<RES>, CallError>
    where
        REQ: Serialize,
        RES: de::DeserializeOwned,
    {
        // Serialize
        let topic = type_name::<REQ>();
        let data = match format {
            SerializationFormat::Bincode => match bincode::serialize(&request) {
                Ok(a) => a,
                Err(err) => {
                    debug!(
                        "failed to serialize the request object (type={}, format={}) - {}",
                        type_name::<REQ>(),
                        format,
                        err
                    );
                    return Err(CallError::SerializationFailed);
                }
            },
            SerializationFormat::Json => match serde_json::to_vec(&request) {
                Ok(a) => a,
                Err(err) => {
                    debug!(
                        "failed to serialize the request object (type={}, format={}) - {}",
                        type_name::<REQ>(),
                        format,
                        err
                    );
                    return Err(CallError::SerializationFailed);
                }
            },
        };

        let (rx, handle) =
            self.marker
                .thread
                .call_internal(Some(self.handle.handle()), topic.to_string(), data);
        Ok(AsyncWasmBusResult::new(
            &self.marker.thread,
            rx,
            handle,
            format,
            ctx,
        ))
    }
}
