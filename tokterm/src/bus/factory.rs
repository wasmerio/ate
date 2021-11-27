use std::collections::HashMap;
use wasm_bus::abi::CallHandle;

use super::*;

pub struct BusFactory {
    sessions: HashMap<CallHandle, Box<dyn Session>>,
}

impl BusFactory {
    pub fn new() -> BusFactory {
        BusFactory {
            sessions: HashMap::default(),
        }
    }

    pub fn start(&mut self, handle: CallHandle, wapm: &str, topic: &str, request: &Vec<u8>) -> Box<dyn Invokable> {
        match super::builtin::builtin(wapm, topic, request){
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
