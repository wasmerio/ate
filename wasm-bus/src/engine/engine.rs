#![allow(dead_code)]
use once_cell::sync::Lazy;
use serde::*;
use std::borrow::Cow;
#[allow(unused_imports)]
use std::future::Future;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;
use std::sync::{Arc, MutexGuard};
use std::task::{Context, Waker};
use std::{collections::HashMap, collections::HashSet, sync::Mutex};
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::abi::*;

static GLOBAL_ENGINE: Lazy<BusEngine> = Lazy::new(|| BusEngine::default());

#[derive(Default)]
pub struct BusEngineState {
    pub handles: HashSet<CallHandle>,
    pub calls: HashMap<CallHandle, Arc<dyn CallOps>>,
    pub callbacks: HashMap<CallHandle, Arc<dyn FinishOps>>,
    #[cfg(feature = "rt")]
    pub listening: HashMap<String, ListenService>,
    #[cfg(feature = "rt")]
    pub respond_to: HashMap<String, RespondToService>,
}

#[derive(Default)]
pub struct BusEngine {
    state: RwLock<BusEngineState>,
    wakers: Mutex<HashMap<CallHandle, Waker>>,
}

impl BusEngine {
    fn read<'a>() -> RwLockReadGuard<'a, BusEngineState> {
        GLOBAL_ENGINE.state.read().unwrap()
    }

    fn write<'a>() -> RwLockWriteGuard<'a, BusEngineState> {
        GLOBAL_ENGINE.state.write().unwrap()
    }

    fn try_write<'a>() -> Option<RwLockWriteGuard<'a, BusEngineState>> {
        GLOBAL_ENGINE.state.try_write().ok()
    }

    fn wakers<'a>() -> MutexGuard<'a, HashMap<CallHandle, Waker>> {
        GLOBAL_ENGINE.wakers.lock().unwrap()
    }

    // This function will block
    #[cfg(feature = "rt")]
    pub fn start(
        topic: String,
        parent: Option<CallHandle>,
        handle: CallHandle,
        request: Vec<u8>,
    ) -> Result<(), CallError> {
        let state = BusEngine::read();
        if let Some(parent) = parent {
            if let Some(respond_to) = state.respond_to.get(&topic) {
                let respond_to = respond_to.clone();
                drop(state);

                let mut state = BusEngine::write();
                if state.handles.contains(&handle) == false {
                    state.handles.insert(handle);

                    respond_to.process(parent, handle, request);
                    return Ok(());
                } else {
                    return Err(CallError::InvalidHandle);
                }
            } else {
                return Err(CallError::InvalidHandle);
            }
        } else if let Some(listen) = state.listening.get(&topic) {
            let listen = listen.clone();
            drop(state);

            let mut state = BusEngine::write();
            if state.handles.contains(&handle) == false {
                state.handles.insert(handle);

                listen.process(handle, request);
                return Ok(());
            } else {
                return Err(CallError::InvalidHandle);
            }
        } else {
            return Err(CallError::InvalidTopic);
        }
    }

    // This function will block
    pub fn finish(handle: CallHandle, response: Vec<u8>) {
        {
            let state = BusEngine::read();
            if let Some(call) = state.calls.get(&handle) {
                let call = Arc::clone(call);
                drop(state);
                call.data(response);
            } else if let Some(callback) = state.callbacks.get(&handle) {
                let callback = Arc::clone(callback);
                drop(state);
                let _ = callback.process(response);
            }
        };

        let mut wakers = Self::wakers();
        if let Some(waker) = wakers.remove(&handle) {
            drop(wakers);
            waker.wake();
        }
    }

    pub fn error(handle: CallHandle, err: CallError) {
        {
            let mut state = BusEngine::write();
            if let Some(call) = state.calls.remove(&handle) {
                drop(state);
                call.error(err);
            }
        }

        {
            let mut wakers = Self::wakers();
            if let Some(waker) = wakers.remove(&handle) {
                drop(wakers);
                waker.wake();
            }
        }
    }

    pub fn subscribe(handle: &CallHandle, cx: &mut Context<'_>) {
        let waker = cx.waker().clone();
        let mut wakers = Self::wakers();
        wakers.insert(handle.clone(), waker);
    }

    pub fn remove(handle: &CallHandle) {
        if let Some(mut state) = BusEngine::try_write() {
            #[cfg(feature = "rt")]
            for respond_to in state.respond_to.values_mut() {
                respond_to.remove(handle);
            }
            state.handles.remove(handle);
            if let Some(delayed_remove) = state.calls.remove(handle) {
                drop(state);
                drop(delayed_remove);
            } else if let Some(delayed_remove) = state.callbacks.remove(handle) {
                drop(state);
                drop(delayed_remove);
            }
        }

        let mut wakers = Self::wakers();
        wakers.remove(handle);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn call(
        parent: Option<CallHandle>,
        wapm: Cow<'static, str>,
        topic: Cow<'static, str>,
        format: SerializationFormat,
        session: Option<String>,
    ) -> Call {
        let mut handle: CallHandle = crate::abi::syscall::handle().into();
        let mut call = Call {
            handle,
            parent,
            wapm,
            topic,
            format,
            session,
            state: Arc::new(Mutex::new(CallState { result: None })),
            callbacks: Arc::new(Mutex::new(Vec::new())),
        };

        loop {
            handle = crate::abi::syscall::handle().into();
            call.handle = handle;

            {
                let mut state = BusEngine::write();
                if state.handles.contains(&handle) == false
                    && state.calls.contains_key(&handle) == false
                {
                    state.handles.insert(handle);
                    state.calls.insert(handle, Arc::new(call.clone()));
                    return call;
                }
            }
            std::thread::yield_now();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn call(
        _parent: Option<CallHandle>,
        _wapm: Cow<'static, str>,
        _topic: Cow<'static, str>,
        _format: SerializationFormat,
        _session: Option<String>,
    ) -> Call {
        panic!("call not supported on this platform");
    }

    #[cfg(target_arch = "wasm32")]
    pub fn callback<RES, REQ, F>(format: SerializationFormat, mut callback: F) -> Finish
    where
        REQ: de::DeserializeOwned + Send + Sync + 'static,
        RES: Serialize + Send + Sync + 'static,
        F: FnMut(REQ) -> Result<RES, CallError>,
        F: Send + 'static,
    {
        let callback = move |req: Vec<u8>| {
            let req = match format {
                SerializationFormat::Bincode => bincode::deserialize::<REQ>(req.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
                SerializationFormat::Json => serde_json::from_slice::<REQ>(req.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
            };

            let res = callback(req)?;

            let res = match format {
                SerializationFormat::Bincode => bincode::serialize::<RES>(&res)
                    .map_err(|_err| CallError::SerializationFailed)?,
                SerializationFormat::Json => serde_json::to_vec::<RES>(&res)
                    .map_err(|_err| CallError::SerializationFailed)?,
            };

            Ok(res)
        };
        BusEngine::register(callback)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn callback<RES, REQ, F>(_format: SerializationFormat, _callback: F) -> Finish
    where
        REQ: de::DeserializeOwned + Send + Sync + 'static,
        RES: Serialize + Send + Sync + 'static,
        F: FnMut(REQ) -> Result<RES, CallError>,
        F: Send + 'static,
    {
        panic!("recv not supported on this platform");
    }

    #[cfg(target_arch = "wasm32")]
    fn register<F>(callback: F) -> Finish
    where
        F: FnMut(Vec<u8>) -> Result<Vec<u8>, CallError>,
        F: Send + 'static,
    {
        let mut handle: CallHandle = crate::abi::syscall::handle().into();
        let mut recv = Finish {
            handle: handle,
            callback: Arc::new(Mutex::new(Box::new(callback))),
        };

        loop {
            handle = crate::abi::syscall::handle().into();
            recv.handle = handle;

            {
                let mut state = BusEngine::write();
                if state.handles.contains(&handle) == false
                    && state.callbacks.contains_key(&handle) == false
                {
                    state.handles.insert(handle);
                    state.callbacks.insert(handle, Arc::new(recv.clone()));
                    return recv;
                }
            }
            std::thread::yield_now();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn register<F>(_callback: F) -> Finish
    where
        F: FnMut(Vec<u8>) -> Result<(), CallError>,
    {
        panic!("recv not supported on this platform");
    }

    #[cfg(feature = "rt")]
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn listen_internal<F, Fut>(topic: String, callback: F)
    where
        F: Fn(CallHandle, Vec<u8>) -> Result<Fut, CallError>,
        F: Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<u8>, CallError>>,
        Fut: Send + 'static,
    {
        {
            let mut state = BusEngine::write();
            state.listening.insert(
                topic.clone(),
                ListenService::new(Arc::new(move |handle, req| {
                    let res = callback(handle, req);
                    Box::pin(async move { Ok(res?.await?) })
                })),
            );
        }

        crate::abi::syscall::listen(topic.as_str());
    }

    #[cfg(feature = "rt")]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn listen_internal<F, Fut>(_topic: String, _callback: F)
    where
        F: Fn(CallHandle, Vec<u8>) -> Result<Fut, CallError>,
        F: Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<u8>, CallError>>,
        Fut: Send + 'static,
    {
        panic!("listen not supported on this platform");
    }

    #[cfg(feature = "rt")]
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn respond_to_internal<F, Fut>(topic: String, parent: CallHandle, callback: F)
    where
        F: Fn(CallHandle, Vec<u8>) -> Result<Fut, CallError>,
        F: Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<u8>, CallError>>,
        Fut: Send + 'static,
    {
        {
            let mut state = BusEngine::write();
            if state.respond_to.contains_key(&topic) == false {
                state
                    .respond_to
                    .insert(topic.clone(), RespondToService::default());
                crate::abi::syscall::listen(topic.as_str());
            }
            let respond_to = state.respond_to.get_mut(&topic).unwrap();
            respond_to.add(
                parent,
                Arc::new(move |handle, req| {
                    let res = callback(handle, req);
                    Box::pin(async move { Ok(res?.await?) })
                }),
            );
        }
    }

    #[cfg(feature = "rt")]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn respond_to_internal<F, Fut>(_topic: String, _parent: CallHandle, _callback: F)
    where
        F: Fn(CallHandle, Vec<u8>) -> Result<Fut, CallError>,
        F: Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<u8>, CallError>>,
        Fut: Send + 'static,
    {
        panic!("respond_to not supported on this platform");
    }
}
