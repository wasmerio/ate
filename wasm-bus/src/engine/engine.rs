use std::{collections::HashMap, sync::Mutex};
use std::{collections::HashSet};
use std::task::{Context, Waker};

use crate::abi::*;

use once_cell::sync::Lazy;

pub(super) static BUS_ENGINE: Lazy<BusEngine> =
    Lazy::new(|| BusEngine::default());

#[derive(Debug, Default)]
pub struct BusEngineState
{
    pub handles: HashSet<CallHandle>,
    pub calls: HashMap<CallHandle, Data>,
    pub wakers: HashMap<CallHandle, Waker>
}

#[derive(Debug, Default)]
pub struct BusEngine
{
    state: Mutex<BusEngineState>,
}

impl BusEngine
{
    // This function will block
    pub fn put(&self, handle: CallHandle, response: Data)
    {
        let mut state = self.state.lock().unwrap();
        state.calls.insert(handle, response);
        if let Some(waker) = state.wakers.remove(&handle) {
            waker.wake();
        }
    }

    // This function is none blocking
    pub fn get(&self, handle: &CallHandle, mut cx: Option<&mut Context<'_>>) -> Option<Data>
    {
        let mut state = self.state.lock().unwrap();
        if let Some(ret) = state.calls.remove(handle) {
            state.handles.remove(handle);
            state.wakers.remove(handle);
            return Some(ret);
        } else {
            if let Some(cx) = cx.as_mut() {
                state.wakers.insert(handle.clone(), cx.waker().clone());
            }
            return None;
        }
    }

    pub fn generate(&self) -> CallHandle {
        loop {
            let handle = CallHandle {
                id: crate::abi::syscall::rand(),
            };
            if let Ok(mut state) = self.state.try_lock() {
                if state.handles.contains(&handle) == false {
                    state.handles.insert(handle.clone());
                    return handle;
                }
            }
            std::thread::yield_now();
        }
    }
}