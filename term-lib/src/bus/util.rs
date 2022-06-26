use serde::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus::abi::SerializationFormat;
use wasmer_vbus::BusDataFormat;
use wasmer_vbus::VirtualBusError;

pub fn conv_error(fault: VirtualBusError) -> BusError {
    use VirtualBusError::*;
    match fault {
        Serialization => BusError::SerializationFailed,
        Deserialization => BusError::DeserializationFailed,
        InvalidWapm => BusError::InvalidWapm,
        FetchFailed => BusError::FetchFailed,
        CompileError => BusError::CompileError,
        InvalidABI => BusError::IncorrectAbi,
        Aborted => BusError::Aborted,
        BadHandle => BusError::InvalidHandle,
        InvalidTopic => BusError::InvalidTopic,
        BadCallback => BusError::MissingCallbacks,
        Unsupported => BusError::Unsupported,
        BadRequest => BusError::BadRequest,
        AccessDenied => BusError::AccessDenied,
        InternalError => BusError::InternalFailure,
        MemoryAllocationFailed => BusError::MemoryAllocationFailed,
        InvokeFailed => BusError::BusInvocationFailed,
        AlreadyConsumed => BusError::AlreadyConsumed,
        MemoryAccessViolation => BusError::MemoryAccessViolation,
        UnknownError => BusError::Unknown,
    }
}

pub fn conv_error_back(fault: BusError) -> VirtualBusError {
    use VirtualBusError::*;
    match fault {
        BusError::SerializationFailed => Serialization,
        BusError::DeserializationFailed => Deserialization,
        BusError::InvalidWapm => InvalidWapm,
        BusError::FetchFailed => FetchFailed,
        BusError::CompileError => CompileError,
        BusError::IncorrectAbi => InvalidABI,
        BusError::Aborted => Aborted,
        BusError::InvalidHandle => BadHandle,
        BusError::InvalidTopic => InvalidTopic,
        BusError::MissingCallbacks => BadCallback,
        BusError::Unsupported => Unsupported,
        BusError::BadRequest => BadRequest,
        BusError::AccessDenied => AccessDenied,
        BusError::InternalFailure => InternalError,
        BusError::MemoryAllocationFailed => MemoryAllocationFailed,
        BusError::BusInvocationFailed => InvokeFailed,
        BusError::AlreadyConsumed => AlreadyConsumed,
        BusError::MemoryAccessViolation => MemoryAccessViolation,
        BusError::Unknown => UnknownError,
        BusError::Success => UnknownError,
    }
}

pub fn conv_format(format: BusDataFormat) -> SerializationFormat {
    use BusDataFormat::*;
    match format {
        Raw => SerializationFormat::Raw,
        Bincode => SerializationFormat::Bincode,
        MessagePack => SerializationFormat::MessagePack,
        Json => SerializationFormat::Json,
        Yaml => SerializationFormat::Yaml,
        Xml => SerializationFormat::Xml
    }
}

pub fn conv_format_back(format: SerializationFormat) -> BusDataFormat {
    use BusDataFormat::*;
    match format {
        SerializationFormat::Raw => Raw,
        SerializationFormat::Bincode => Bincode,
        SerializationFormat::MessagePack => MessagePack,
        SerializationFormat::Json => Json,
        SerializationFormat::Yaml => Yaml,
        SerializationFormat::Xml => Xml
    }
}

pub fn decode_request<T>(format: BusDataFormat, request: Vec<u8>) -> Result<T, BusError>
where
    T: de::DeserializeOwned,
{
    let format = conv_format(format);
    format.deserialize(request)
}

pub fn encode_response<T>(format: BusDataFormat, response: &T) -> Result<Vec<u8>, BusError>
where
    T: Serialize,
{
    let format = conv_format(format);
    format.serialize(response)
}
