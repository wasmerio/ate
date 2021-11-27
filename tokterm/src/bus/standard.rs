use std::any::type_name;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend;
use wasm_bus::abi::CallError;
use std::collections::HashMap;

use super::*;

#[derive(Debug, Clone)]
pub struct StandardBus
{
    ws_factory: WebSocketFactory
}

impl StandardBus
{
    pub fn new() -> StandardBus {
        StandardBus {
            ws_factory: WebSocketFactory::new()
        }
    }

    pub fn create(&self, wapm: &str, topic: &str, request: &Vec<u8>, client_callbacks: HashMap<String, WasmBusFeeder>) -> Result<(Box<dyn Invokable>, Box<dyn Session>), CallError> {
        match (wapm, topic) {
            ("os", topic) if topic == type_name::<backend::ws::Connect>() => {
                let request = decode_request(request.as_ref())?;
                
                let (invoker, session) = self.ws_factory.create(request, client_callbacks)?;
                Ok((Box::new(invoker), Box::new(session)))
            }
            _ => Err(CallError::InvalidTopic),
        }
    }
}