use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;

use super::*;

#[derive(Clone)]
pub struct BusFactory {
    sessions: Arc<Mutex<HashMap<CallHandle, Arc<dyn Invokable>>>>,
}

impl BusFactory {
    pub fn new() -> BusFactory {
        BusFactory {
            sessions: Arc::new(Mutex::new(HashMap::default())),
        }
    }

    pub fn start(&self, handle: CallHandle, wapm: &str, topic: &str) -> Arc<dyn Invokable> {
        if let Some(invoker) = super::builtin::builtin(wapm, topic) {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.insert(handle, Arc::clone(&invoker));
            return invoker;
        }

        ErrornousInvokable::new(CallError::InvalidWapm)
    }

    pub fn get(&self, handle: CallHandle) -> Option<Arc<dyn Invokable>> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(&handle).map(|a| a.clone())
    }

    pub fn close(&self, handle: CallHandle) -> Option<Arc<dyn Invokable>> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(&handle)
    }
}
