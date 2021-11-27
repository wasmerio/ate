use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;

pub fn decode_request<T>(request: &[u8]) -> Result<T, CallError>
where T: de::DeserializeOwned
{
    let req: T = match bincode::deserialize(request) {
        Ok(a) => a,
        Err(err) => {
            warn!("failed to deserialize bus call - {}", err);
            return Err(CallError::DeserializationFailed);
        }
    };
    Ok(req)
}

pub fn encode_response<T>(response: &T) -> Result<Vec<u8>, CallError>
where T: Serialize
{
    let res = match bincode::serialize(response) {
        Ok(a) => a,
        Err(err) => {
            warn!("failed to serialize bus call response - {}", err);
            return Err(CallError::SerializationFailed);
        }
    };
    Ok(res)
}