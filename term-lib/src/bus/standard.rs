use std::any::type_name;
use std::collections::HashMap;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::backend;

use crate::api::System;

use super::*;

#[derive(Debug, Clone)]
pub struct StandardBus {
    system: System,
    process_factory: ProcessExecFactory,
}

impl StandardBus {
    pub fn new(process_factory: ProcessExecFactory) -> StandardBus {
        StandardBus {
            system: System::default(),
            process_factory,
        }
    }

    pub async fn create(
        &self,
        wapm: &str,
        topic: &str,
        request: &Vec<u8>,
        client_callbacks: &HashMap<String, WasmBusCallback>,
    ) -> Result<(Box<dyn Invokable>, Option<Box<dyn Session>>), CallError> {
        match (wapm, topic) {
            ("os", topic) if topic == type_name::<backend::ws::Connect>() => {
                let request = decode_request(request.as_ref())?;

                let (invoker, session) = ws::web_socket(request, client_callbacks.clone())?;
                Ok((Box::new(invoker), Some(Box::new(session))))
            }
            ("os", topic) if topic == type_name::<backend::time::Sleep>() => {
                let request = decode_request(request.as_ref())?;
                let invoker = time::sleep(self.system, request);
                Ok((Box::new(invoker), None))
            }
            ("os", topic) if topic == type_name::<backend::reqwest::Request>() => {
                let request = decode_request(request.as_ref())?;
                let invoker = reqwest::reqwest(self.system, request);
                Ok((Box::new(invoker), None))
            }
            ("os", topic) if topic == type_name::<backend::process::Spawn>() => {
                let request = decode_request(request.as_ref())?;

                let created = self
                    .process_factory
                    .eval(request, client_callbacks.clone())
                    .await?;
                Ok((Box::new(created.invoker), Some(Box::new(created.session))))
            }
            _ => Err(CallError::InvalidTopic),
        }
    }
}
