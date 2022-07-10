#![allow(dead_code)]
use once_cell::sync::Lazy;
#[allow(unused_imports, dead_code)]
use std::any::type_name;
use std::borrow::Cow;
use std::convert::TryInto;
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
    pub children: HashMap<CallHandle, Vec<CallHandle>>,
    #[cfg(feature = "rt")]
    pub listening: HashMap<u128, ListenService>,
    #[cfg(feature = "rt")]
    pub respond_to: HashMap<u128, RespondToService>,
}

#[derive(Default)]
pub struct BusEngine {
    state: RwLock<BusEngineState>,
    wakers: Mutex<HashMap<CallHandle, Waker>>,
}

// Function that hashes the topic using SHA256
pub(crate) fn hash_topic(topic: &Cow<'static, str>) -> u128 {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&topic.bytes().collect::<Vec<_>>());
    let hash: [u8; 16] = hasher.finalize()[..16].try_into().unwrap();
    u128::from_le_bytes(hash)
}

impl BusEngine {
    pub(crate) fn read<'a>() -> RwLockReadGuard<'a, BusEngineState> {
        GLOBAL_ENGINE.state.read().unwrap()
    }

    pub(crate) fn write<'a>() -> RwLockWriteGuard<'a, BusEngineState> {
        GLOBAL_ENGINE.state.write().unwrap()
    }

    pub(crate) fn try_write<'a>() -> Option<RwLockWriteGuard<'a, BusEngineState>> {
        GLOBAL_ENGINE.state.try_write().ok()
    }

    fn wakers<'a>() -> MutexGuard<'a, HashMap<CallHandle, Waker>> {
        GLOBAL_ENGINE.wakers.lock().unwrap()
    }

    // This function will block
    #[cfg(feature = "rt")]
    pub fn start(
        topic_hash: u128,
        parent: Option<CallHandle>,
        handle: CallHandle,
        request: Vec<u8>,
        format: SerializationFormat,
    ) -> Result<(), BusError> {
        let state = BusEngine::read();
        if let Some(parent) = parent {
            if let Some(parent) = state.calls.get(&parent) {
                // If the callback is registered then process it and finish the call
                if parent.callback(topic_hash, request, format) != CallbackResult::InvalidTopic {
                    // The topic exists at least - so lets close the handle
                    syscall::call_close(handle);   
                    return Ok(());
                } else {
                    return Err(BusError::InvalidTopic);    
                }
            }

            if let Some(respond_to) = state.respond_to.get(&topic_hash) {
                let respond_to = respond_to.clone();
                drop(state);

                let mut state = BusEngine::write();
                if state.handles.contains(&handle) == false {
                    state.handles.insert(handle);
                    drop(state);

                    crate::task::spawn(async move {
                        respond_to.process(parent, handle, request, format).await;
                    });
                    return Ok(());
                } else {
                    return Err(BusError::InvalidHandle);
                }
            } else {
                return Err(BusError::InvalidHandle);
            }
        } else if let Some(listen) = state.listening.get(&topic_hash) {
            let listen = listen.clone();
            drop(state);

            let mut state = BusEngine::write();
            if state.handles.contains(&handle) == false {
                state.handles.insert(handle);
                drop(state);

                crate::task::spawn(async move {
                    listen.process(handle, request, format).await;
                });
                return Ok(());
            } else {
                return Err(BusError::InvalidHandle);
            }
        } else {
            return Err(BusError::InvalidTopic);
        }
    }

    // This function will block
    pub fn finish_callback(
        topic_hash: u128,
        handle: CallHandle,
        response: Vec<u8>,
        format: SerializationFormat,
    ) {
        {
            let state = BusEngine::read();
            if let Some(call) = state.calls.get(&handle) {
                let call = Arc::clone(call);
                drop(state);
                trace!(
                    "wasmer_bus_callback (handle={}, response={} bytes)",
                    handle.id,
                    response.len()
                );
                call.callback(topic_hash, response, format);
            } else {
                trace!(
                    "wasmer_bus_callback (handle={}, response={} bytes, orphaned)",
                    handle.id,
                    response.len(),
                );
            }
        };

        let mut wakers = Self::wakers();
        if let Some(waker) = wakers.remove(&handle) {
            drop(wakers);
            waker.wake();
        }
    }

    // This function will block
    pub fn result(
        handle: CallHandle,
        response: Vec<u8>,
        format: SerializationFormat,
    ) {
        {
            let mut state = BusEngine::write();
            if let Some(call) = state.calls.remove(&handle) {
                drop(state);
                trace!(
                    "wasmer_bus_finish (handle={}, response={} bytes)",
                    handle.id,
                    response.len()
                );
                call.data(response, format);
            } else {
                trace!(
                    "wasmer_bus_finish (handle={}, response={} bytes, orphaned)",
                    handle.id,
                    response.len()
                );
            }
        };

        let mut wakers = Self::wakers();
        if let Some(waker) = wakers.remove(&handle) {
            drop(wakers);
            waker.wake();
        }
    }

    pub fn error(handle: CallHandle, err: BusError) {
        {
            let mut state = BusEngine::write();
            if let Some(call) = state.calls.remove(&handle) {
                drop(state);
                trace!(
                    "wasmer_bus_err (handle={}, error={})",
                    handle.id,
                    err,
                );
                call.error(err);
            } else {
                trace!(
                    "wasmer_bus_err (handle={}, error={}, orphaned)",
                    handle.id,
                    err
                );
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

    pub fn add_callback(handle: CallHandle, child: CallHandle) {
        let mut state = BusEngine::write();
        let children = state.children
            .entry(handle)
            .or_insert(Vec::new());
        children.push(child);
    }

    pub fn close(handle: &CallHandle, reason: &'static str) {
        let mut children = Vec::new();
        {
            let mut delayed_drop1 = Vec::new();
            let mut delayed_drop2 = Vec::new();
            
            {
                let mut state = BusEngine::write();
                #[cfg(feature = "rt")]
                state.handles.remove(handle);
                if let Some(mut c) = state.children.remove(handle) {
                    children.append(&mut c);
                }
                if let Some(drop_me) = state.calls.remove(handle) {
                    trace!(
                        "wasmer_bus_drop (handle={}, reason='{}')",
                        handle.id,
                        reason
                    );
                    delayed_drop2.push(drop_me);
                } else {
                    trace!(
                        "wasmer_bus_drop (handle={}, reason='{}', orphaned)",
                        handle.id,
                        reason
                    );
                }
                for respond_to in state.respond_to.values_mut() {
                    if let Some(drop_me) = respond_to.remove(handle) {
                        delayed_drop1.push(drop_me);
                    }
                }
            }
        }

        for child in children {
            Self::close(&child, reason);
        }

        let mut wakers = Self::wakers();
        wakers.remove(handle);
    }

    #[cfg(feature = "rt")]
    pub(crate) fn listen_internal<F, Fut>(
        format: SerializationFormat,
        topic: String,
        callback: F,
    ) where
        F: Fn(CallHandle, Vec<u8>) -> Result<Fut, BusError>,
        F: Send + Sync + 'static,
        Fut: Future<Output = ListenAction>,
        Fut: Send + 'static,
    {
        let topic_hash = crate::engine::hash_topic(&topic.into());
        {
            let mut state = BusEngine::write();
            state.listening.insert(
                topic_hash,
                ListenService::new(
                    format,
                    Arc::new(move |handle, req| {
                        let res = callback(handle, req);
                        Box::pin(async move {
                            match res {
                                Ok(res) => res.await,
                                Err(err) => ListenAction::Fault(err)
                            }
                        })
                    }),
                ),
            );
        }
    }

    #[cfg(feature = "rt")]
    pub(crate) fn respond_to_internal<F, Fut>(
        format: SerializationFormat,
        topic: String,
        parent: CallHandle,
        callback: F,
    ) where
        F: Fn(CallHandle, Vec<u8>) -> Result<Fut, BusError>,
        F: Send + Sync + 'static,
        Fut: Future<Output = RespondAction>,
        Fut: Send + 'static,
    {
        let topic_hash = crate::engine::hash_topic(&topic.into());
        {
            let mut state = BusEngine::write();
            if state.respond_to.contains_key(&topic_hash) == false {
                state
                    .respond_to
                    .insert(topic_hash, RespondToService::new(format));
            }
            let respond_to = state.respond_to.get_mut(&topic_hash).unwrap();
            respond_to.add(
                parent,
                Arc::new(move |handle, req| {
                    let res = callback(handle, req);
                    Box::pin(async move {
                        match res {
                            Ok(res) => res.await,
                            Err(err) => RespondAction::Fault(err)
                        }
                    })
                }),
            );
        }
    }
}
