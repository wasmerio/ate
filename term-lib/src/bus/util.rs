use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus::abi::SerializationFormat;

pub fn decode_request<T>(format: SerializationFormat, request: &[u8]) -> Result<T, BusError>
where
    T: de::DeserializeOwned,
{
    let req: T = match format {
        SerializationFormat::Bincode => match bincode::deserialize(request) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to deserialize bus call - {}", err);
                return Err(BusError::DeserializationFailed);
            }
        },
        SerializationFormat::Json => match serde_json::from_slice(request) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to deserialize bus call - {}", err);
                return Err(BusError::DeserializationFailed);
            }
        },
        _ => return Err(BusError::Unsupported)
    };
    Ok(req)
}

pub fn encode_response<T>(format: SerializationFormat, response: &T) -> Result<Vec<u8>, BusError>
where
    T: Serialize,
{
    let res = match format {
        SerializationFormat::Bincode => match bincode::serialize(response) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to serialize bus call response - {}", err);
                return Err(BusError::SerializationFailed);
            }
        },
        SerializationFormat::Json => match serde_json::to_vec(response) {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to serialize bus call response - {}", err);
                return Err(BusError::SerializationFailed);
            }
        },
        _ => return Err(BusError::Unsupported)
    };
    Ok(res)
}
