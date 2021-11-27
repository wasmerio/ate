use std::collections::HashMap;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallHandle;

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

    pub fn start(
        &mut self,
        handle: CallHandle,
        wapm: &str,
        topic: &str,
        request: &Vec<u8>,
        client_callbacks: HashMap<String, WasmBusFeeder>,
    ) -> Box<dyn Invokable> {
        match self.standard.create(wapm, topic, request, client_callbacks) {
            Ok((invoker, Some(session))) => {
                self.sessions.insert(handle, session);
                invoker
            }
            Ok((invoker, None)) => invoker,
            Err(err) => ErrornousInvokable::new(err),
        }
    }

    pub fn get(&mut self, handle: CallHandle) -> Option<&mut Box<dyn Session>> {
        self.sessions.get_mut(&handle)
    }

    pub fn close(&mut self, handle: CallHandle) -> Option<Box<dyn Session>> {
        self.sessions.remove(&handle)
    }
}
