use std::collections::HashMap;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;

use super::*;

// A BUS factory is created for every running process and allows them
// to spawn operating system commands and/or other sub processes
pub struct BusFactory {
    standard: StandardBus,
    sub_processes: SubProcessFactory,
    sessions: HashMap<CallHandle, Box<dyn Session>>,
}

impl BusFactory {
    pub fn new(process_factory: ProcessExecFactory) -> BusFactory {
        BusFactory {
            standard: StandardBus::new(process_factory.clone()),
            sub_processes: SubProcessFactory::new(process_factory),
            sessions: HashMap::default(),
        }
    }

    pub fn start(
        &mut self,
        handle: CallHandle,
        wapm: &str,
        topic: &str,
        request: &Vec<u8>,
        client_callbacks: HashMap<String, WasmBusCallback>,
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
        match self.sub_processes.get_or_create(wapm) {
            Ok(sub_process) => match sub_process.create(topic, request, client_callbacks) {
                Ok((invoker, Some(session))) => {
                    self.sessions.insert(handle, session);
                    invoker
                }
                Ok((invoker, None)) => invoker,
                Err(err) => ErrornousInvokable::new(err),
            },
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
