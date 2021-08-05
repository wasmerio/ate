#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;
use serde_json::Error as JsonError;

#[derive(Debug)]
pub enum SerializationError
{
    NoPrimarykey,
    NoData,
    InvalidSerializationFormat,
    IO(tokio::io::Error),
    EncodeError(RmpEncodeError),
    DecodeError(RmpDecodeError),
    JsonError(JsonError),
    BincodeError(bincode::Error),
    #[allow(dead_code)]
    CollectionDetached,
    SerdeError(String),
    WeakDio,
    SaveParentFirst,
}

impl From<RmpEncodeError>
for SerializationError {
    fn from(err: RmpEncodeError) -> SerializationError {
        SerializationError::EncodeError(err)
    }
}

impl From<tokio::io::Error>
for SerializationError
{
    fn from(err: tokio::io::Error) -> SerializationError {
        SerializationError::IO(err)
    }   
}

impl From<bincode::Error>
for SerializationError
{
    fn from(err: bincode::Error) -> SerializationError {
        SerializationError::BincodeError(err)
    }   
}

impl From<RmpDecodeError>
for SerializationError {
    fn from(err: RmpDecodeError) -> SerializationError {
        SerializationError::DecodeError(err)
    }
}

impl From<JsonError>
for SerializationError {
    fn from(err: JsonError) -> SerializationError {
        SerializationError::JsonError(err)
    }
}

impl std::fmt::Display
for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SerializationError::NoPrimarykey => {
                write!(f, "Data object does not have a primary key")
            },
            SerializationError::NoData => {
                write!(f, "Data object has no actual data")
            },
            SerializationError::InvalidSerializationFormat => {
                write!(f, "Data is stored in an unknown serialization format")
            },
            SerializationError::IO(err) => {
                write!(f, "IO error during serialization - {}", err)
            },
            SerializationError::EncodeError(err) => {
                write!(f, "MessagePack encoding error - {}", err)
            },
            SerializationError::DecodeError(err) => {
                write!(f, "MessagePack decoding error - {}", err)
            },
            SerializationError::JsonError(err) => {
                write!(f, "JSON serialization error - {}", err)
            },
            SerializationError::BincodeError(err) => {
                write!(f, "Bincode serialization error - {}", err)
            },
            SerializationError::CollectionDetached => {
                write!(f, "Collection is detached from a parent")
            },
            SerializationError::SerdeError(err) => {
                write!(f, "Serde error during serialization - {}", err)
            },
            SerializationError::WeakDio => {
                write!(f, "The DIO that created this object has gone out of scope")
            },
            SerializationError::SaveParentFirst => {
                write!(f, "You must save the parent object before attempting to push objects to this vector")
            },
        }
    }
}

impl std::error::Error
for SerializationError
{
}