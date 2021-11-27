use std::collections::HashMap;
use wasm_bus::abi::CallHandle;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;

pub struct BusFactory {
    standard: StandardBus,
    sessions: HashMap<CallHandle, Box<dyn Session>>,
}

impl BusFactory {
    pub fn new(standard: StandardBus) -> BusFactory {
        BusFactory {
            standard,
            sessions: HashMap::default(),
        }
    }

    pub fn start(&mut self, handle: CallHandle, wapm: &str, topic: &str, request: &Vec<u8>, client_callbacks: HashMap<String, WasmBusFeeder>) -> Box<dyn Invokable> {
        match self.standard.create(wapm, topic, request, client_callbacks){
            Ok((invoker, session)) => {
                self.sessions.insert(handle, session);
                invoker
            },
            Err(err) => {
                ErrornousInvokable::new(err)
            }
        }
    }

    pub fn get(&mut self, handle: CallHandle) -> Option<&mut Box<dyn Session>> {
        self.sessions.get_mut(&handle)
    }

    pub fn close(&mut self, handle: CallHandle) -> Option<Box<dyn Session>> {
        self.sessions.remove(&handle)
    }
}
