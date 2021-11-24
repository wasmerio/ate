#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::{collections::HashMap, sync::Mutex};
use std::task::{Context, Waker};
use serde::*;
use std::sync::{Arc, MutexGuard};
use std::marker::PhantomData;
use std::borrow::Cow;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;
use once_cell::sync::Lazy;

use crate::abi::*;

static GLOBAL_ENGINE: Lazy<BusEngine> =
    Lazy::new(|| BusEngine::default());

#[derive(Default)]
pub struct BusEngineState
{
    pub calls: HashMap<CallHandle, Arc<dyn CallOps>>,
}

#[derive(Default)]
pub struct BusEngine
{
    state: RwLock<BusEngineState>,
    wakers: Mutex<HashMap<CallHandle, Waker>>
}

impl BusEngine
{
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
    pub fn put(handle: CallHandle, topic: String, response: Vec<u8>)
    {
        {
            let state = BusEngine::read();
            if let Some(call) = state.calls.get(&handle) {
                let call = Arc::clone(call);
                drop(state);
                call.data(topic, response);
            }
        };

        {
            let mut wakers = Self::wakers();
            if let Some(waker) = wakers.remove(&handle) {
                drop(wakers);
                waker.wake();
            }
        }
    }

    pub fn error(handle: CallHandle, err: CallError)
    {
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

    pub fn subscribe(handle: &CallHandle, cx: &mut Context<'_>)
    {
        let waker = cx.waker().clone();
        let mut wakers = Self::wakers();
        wakers.insert(handle.clone(), waker);
    }

    pub fn remove(handle: &CallHandle) {
        if let Some(mut state) = BusEngine::try_write() {
            let delayed_remove = state.calls.remove(handle);
            drop(state);
            drop(delayed_remove);
        }

        let mut wakers = Self::wakers();
        wakers.remove(handle);
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
                let mut state = BusEngine::write();
                if state.calls.contains_key(&handle) == false {
                    state.calls.insert(handle, Arc::new(call.clone()));
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
    where REQ: de::DeserializeOwned + Send + Sync + 'static,
          RES: Serialize + Send + Sync + 'static
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
                let state = BusEngine::write();
                if state.calls.contains_key(&handle) == false {
                    //state.calls.insert(handle, Box::new(recv.clone()));
                    return recv;
                }
            }
            std::thread::yield_now();
        }
    }
}