use std::any::type_name;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend;
use wasm_bus::abi::CallError;

use super::*;

pub fn builtin(wapm: &str, topic: &str, request: &Vec<u8>) -> Result<(Box<dyn Invokable>, Box<dyn Session>), CallError> {
    match (wapm, topic) {
        ("os", topic) if topic == type_name::<backend::ws::Connect>() => {
            let request = decrypt_request(request.as_ref())?;
            
            let (invoker, session) = WebSocket::new(request)?;
            Ok((Box::new(invoker), Box::new(session)))
        }
        _ => Err(CallError::InvalidTopic),
    }
}