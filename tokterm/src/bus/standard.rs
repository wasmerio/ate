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
    reqwest_factory: WebRequestFactory,
    process_factory: ProcessExecFactory,
}

impl StandardBus {
    pub fn new(process_factory: ProcessExecFactory) -> StandardBus {
        StandardBus {
            ws_factory: WebSocketFactory::new(),
            time_factory: TimeFactory::new(),
            reqwest_factory: WebRequestFactory::new(),
            process_factory
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
            ("os", topic) if topic == type_name::<backend::reqwest::Request>() => {
                let request = decode_request(request.as_ref())?;
                let invoker = self.reqwest_factory.create(request);
                Ok((Box::new(invoker), None))
            }
            ("os", topic) if topic == type_name::<backend::process::Spawn>() => {
                let request = decode_request(request.as_ref())?;

                let (invoker, session) = self.process_factory.create(request, client_callbacks)?;
                Ok((invoker, session))
            }
            _ => Err(CallError::InvalidTopic),
        }
    }
}
