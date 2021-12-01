use std::collections::HashMap;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;

use super::*;

pub struct BusFactory {
    standard: StandardBus,
    sub_processes: SubProcessFactory,
    sessions: HashMap<CallHandle, Box<dyn Session>>,
}

impl BusFactory {
    pub fn new(standard: StandardBus) -> BusFactory {
        BusFactory {
            standard,
            sub_processes: SubProcessFactory::new(),
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
        // The standard bus allows for things like web sockets, http requests, etc...
        match self
            .standard
            .create(wapm, topic, request, &client_callbacks)
        {
            Ok((invoker, Some(session))) => {
                self.sessions.insert(handle, session);
                return invoker;
            }
            Ok((invoker, None)) => {
                return invoker;
            }
            Err(CallError::InvalidTopic) => { /* fall through */ }
            Err(err) => return ErrornousInvokable::new(err),
        }

        // Now we need to check if there is a sub process we can invoke
        if let Some(sub_process) = self.sub_processes.get_or_create(wapm) {
            match sub_process.create(topic, request, client_callbacks) {
                Ok((invoker, Some(session))) => {
                    self.sessions.insert(handle, session);
                    return invoker;
                }
                Ok((invoker, None)) => {
                    return invoker;
                }
                Err(CallError::InvalidTopic) => { /* fall through */ }
                Err(err) => return ErrornousInvokable::new(err),
            }
        }

        // Ok time to give up
        return ErrornousInvokable::new(CallError::InvalidTopic);
    }

    pub fn get(&mut self, handle: CallHandle) -> Option<&mut Box<dyn Session>> {
        self.sessions.get_mut(&handle)
    }

    pub fn close(&mut self, handle: CallHandle) -> Option<Box<dyn Session>> {
        self.sessions.remove(&handle)
    }
}
