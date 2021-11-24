#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::{collections::HashMap, sync::Mutex};
use std::task::{Context, Waker};
use serde::*;
use std::sync::Arc;
use std::marker::PhantomData;
use std::borrow::Cow;
use std::sync::RwLock;
use std::sync::RwLockWriteGuard;
use once_cell::sync::Lazy;

use crate::abi::*;

static GLOBAL_ENGINE: Lazy<BusEngine> =
    Lazy::new(|| BusEngine::default());

#[derive(Default)]
pub struct BusEngineState
{
    pub calls: HashMap<CallHandle, Box<dyn CallOps>>,
    pub wakers: HashMap<CallHandle, Waker>
}

#[derive(Default)]
pub struct BusEngine
{
    state: RwLock<BusEngineState>
}

impl BusEngine
{
    fn local_state<'a>() -> RwLockWriteGuard<'a, BusEngineState> {
        GLOBAL_ENGINE.state.write().unwrap()
    }

    // This function will block
    pub fn put(handle: CallHandle, topic: String, response: Vec<u8>)
    {
        let mut state = BusEngine::local_state();
        if let Some(call) = state.calls.get(&handle) {
            call.data(topic, response);
        }
        if let Some(waker) = state.wakers.remove(&handle) {
            waker.wake();
        }
    }

    pub fn error(handle: CallHandle, err: CallError)
    {
        let mut state = BusEngine::local_state();
        if let Some(call) = state.calls.get(&handle) {
            call.error(err);
        }
        if let Some(waker) = state.wakers.remove(&handle) {
            waker.wake();
        }
    }

    pub fn subscribe(handle: &CallHandle, cx: &mut Context<'_>)
    {
        let mut state = BusEngine::local_state();
        state.wakers.insert(handle.clone(), cx.waker().clone());
    }

    pub fn remove(handle: &CallHandle) {
        let mut state = BusEngine::local_state();
        state.calls.remove(handle);
        state.wakers.remove(handle);    
    }

    pub fn call(wapm: Cow<'static, str>, topic: Cow<'static, str>) -> Call
    {
        let mut handle = CallHandle {
            id: crate::abi::syscall::rand(),
        };
        let mut call = Call {
            handle: handle,
            wapm,
            topic,
            state: Arc::new(RwLock::new(CallState {
                result: None,
                callbacks: HashMap::default()
            })),
        };

        loop {
            handle = CallHandle {
                id: crate::abi::syscall::rand(),
            };
            call.handle = handle;

            {
                let mut state = BusEngine::local_state();
                if state.calls.contains_key(&handle) == false {
                    state.calls.insert(handle, Box::new(call.clone()));
                    return call;
                }
            }
            std::thread::yield_now();
        }
    }

    pub fn call_recursive(handle: CallHandle, wapm: Cow<'static, str>, topic: Cow<'static, str>) -> Call
    {
        Call {
            handle: handle,
            wapm,
            topic,
            state: Arc::new(RwLock::new(CallState {
                result: None,
                callbacks: HashMap::default()
            })),
        }
    }

    pub fn recv<RES, REQ>() -> Recv<RES, REQ>
    where REQ: de::DeserializeOwned + 'static,
          RES: Serialize + 'static
    {
        let topic = std::any::type_name::<REQ>();
        let handle = CallHandle {
            id: crate::abi::syscall::rand(),
        };
        let mut recv = Recv {
            handle: handle,
            topic: topic.into(),
            state: Arc::new(Mutex::new(RecvState {
                response: None,
            })),
            _marker1: PhantomData,
            _marker2: PhantomData
        };

        loop {
            let handle = CallHandle {
                id: crate::abi::syscall::rand(),
            };
            recv.handle = handle;

            {
                let state = BusEngine::local_state();
                if state.calls.contains_key(&handle) == false {
                    return recv;
                }
            }
            std::thread::yield_now();
        }
    }
}