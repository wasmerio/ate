use std::any::type_name;
use std::collections::HashMap;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::backend;

use super::*;

#[derive(Debug, Clone)]
pub struct StandardBus {
    ws_factory: WebSocketFactory,
    time_factory: TimeFactory,
}

impl StandardBus {
    pub fn new() -> StandardBus {
        StandardBus {
            ws_factory: WebSocketFactory::new(),
            time_factory: TimeFactory::new(),
        }
    }

    pub fn create(
        &self,
        wapm: &str,
        topic: &str,
        request: &Vec<u8>,
        client_callbacks: HashMap<String, WasmBusFeeder>,
    ) -> Result<(Box<dyn Invokable>, Option<Box<dyn Session>>), CallError> {
        match (wapm, topic) {
            ("os", topic) if topic == type_name::<backend::ws::Connect>() => {
                let request = decode_request(request.as_ref())?;

                let (invoker, session) = self.ws_factory.create(request, client_callbacks)?;
                Ok((Box::new(invoker), Some(Box::new(session))))
            }
            ("os", topic) if topic == type_name::<backend::time::Sleep>() => {
                let request = decode_request(request.as_ref())?;
                let invoker = self.time_factory.create(request);
                Ok((Box::new(invoker), None))
            }
            _ => Err(CallError::InvalidTopic),
        }
    }
}
