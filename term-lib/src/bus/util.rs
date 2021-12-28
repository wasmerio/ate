use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::SerializationFormat;

pub fn decode_request<T>(format: SerializationFormat, request: &[u8]) -> Result<T, CallError>
where
    T: de::DeserializeOwned,
{
    let req: T = match format {
        SerializationFormat::Bincode => match bincode::deserialize(request) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to deserialize bus call - {}", err);
                return Err(CallError::DeserializationFailed);
            }
        },
        SerializationFormat::Json => match serde_json::from_slice(request) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to deserialize bus call - {}", err);
                return Err(CallError::DeserializationFailed);
            }
        },
    };
    Ok(req)
}

pub fn encode_response<T>(format: SerializationFormat, response: &T) -> Result<Vec<u8>, CallError>
where
    T: Serialize,
{
    let res = match format {
        SerializationFormat::Bincode => match bincode::serialize(response) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to serialize bus call response - {}", err);
                return Err(CallError::SerializationFailed);
            }
        },
        SerializationFormat::Json => match serde_json::to_vec(response) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to serialize bus call response - {}", err);
                return Err(CallError::SerializationFailed);
            }
        },
    };
    Ok(res)
}
