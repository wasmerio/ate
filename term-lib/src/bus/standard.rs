use std::any::type_name;
use std::collections::HashMap;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::SerializationFormat;

use crate::api::System;
use crate::fs::TtyFile;

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
            ("os", topic)
                if topic == type_name::<wasm_bus_ws::api::SocketBuilderConnectRequest>() =>
            {
                let request = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let (invoker, session) = ws::web_socket(request, client_callbacks.clone())?;
                Ok((Box::new(invoker), Some(Box::new(session))))
            }
            ("os", topic) if topic == type_name::<wasm_bus_time::api::TimeSleepRequest>() => {
                let request: wasm_bus_time::api::TimeSleepRequest =
                    decode_request(SerializationFormat::Json, request.as_ref())?;
                let invoker = time::sleep(self.system, request.duration_ms);
                Ok((Box::new(invoker), None))
            }
            ("os", topic) if topic == type_name::<wasm_bus_reqwest::api::ReqwestMakeRequest>() => {
                let request = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let invoker = reqwest::reqwest(self.system, request);
                Ok((Box::new(invoker), None))
            }
            ("os", topic) if topic == type_name::<wasm_bus_process::api::PoolSpawnRequest>() => {
                let request = decode_request(SerializationFormat::Bincode, request.as_ref())?;

                let created = self
                    .process_factory
                    .eval(request, client_callbacks.clone())
                    .await?;
                Ok((Box::new(created.invoker), Some(Box::new(created.session))))
            }
            ("os", topic) if topic == type_name::<wasm_bus_tty::api::TtyStdinRequest>() => {
                let stdio = self.process_factory.exec_factory.stdio();
                let tty = TtyFile::new(&stdio);
                let request = decode_request(SerializationFormat::Json, request.as_ref())?;
                let (invoker, session) = tty::stdin(request, tty, client_callbacks.clone())?;
                Ok((Box::new(invoker), Some(Box::new(session))))
            }
            ("os", topic) if topic == type_name::<wasm_bus_tty::api::TtyStdoutRequest>() => {
                let stdout = self.process_factory.exec_factory.stdout();
                let request = decode_request(SerializationFormat::Json, request.as_ref())?;
                let (invoker, session) = tty::stdout(request, stdout, client_callbacks.clone())?;
                Ok((Box::new(invoker), Some(Box::new(session))))
            }
            ("os", topic) if topic == type_name::<wasm_bus_tty::api::TtyStderrRequest>() => {
                let stderr = self.process_factory.exec_factory.stderr();
                let request = decode_request(SerializationFormat::Json, request.as_ref())?;
                let (invoker, session) = tty::stderr(request, stderr, client_callbacks.clone())?;
                Ok((Box::new(invoker), Some(Box::new(session))))
            }
            ("os", topic) => {
                error!("the os function ({}) is not supported", topic);
                return Err(CallError::Unsupported);
            }
            _ => Err(CallError::InvalidTopic),
        }
    }
}
