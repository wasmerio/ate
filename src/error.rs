use super::crypto::Hash;
extern crate rmp_serde as rmps;

use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;
use serde_json::Error as JsonError;

#[derive(Debug)]
pub enum CryptoError {
    NoIvPresent,    
}

impl From<CryptoError>
for std::io::Error {
    fn from(error: CryptoError) -> Self {
        match error {
            CryptoError::NoIvPresent => std::io::Error::new(std::io::ErrorKind::Other, "The metadata does not have IV component present")
        }
    }
}

impl std::fmt::Display
for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CryptoError::NoIvPresent => {
                write!(f, "The event has no initialization vector")
            },
        }
    }
}

#[derive(Debug)]
pub enum TransformError {
    EncryptionError(openssl::error::ErrorStack),
    IO(std::io::Error),
    CryptoError(CryptoError),
}

impl From<openssl::error::ErrorStack>
for TransformError
{
    fn from(err: openssl::error::ErrorStack) -> TransformError {
        TransformError::EncryptionError(err)
    }
}

impl From<std::io::Error>
for TransformError
{
    fn from(err: std::io::Error) -> TransformError {
        TransformError::IO(err)
    }
}

impl From<CryptoError>
for TransformError
{
    fn from(err: CryptoError) -> TransformError {
        TransformError::CryptoError(err)
    }
}

impl std::fmt::Display
for TransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TransformError::EncryptionError(err) => {
                write!(f, "Encryption error while transforming event data - {}", err)
            },
            TransformError::IO(err) => {
                write!(f, "IO error while transforming event data - {}", err)
            },
            TransformError::CryptoError(err) => {
                write!(f, "Cryptography error while transforming event data - {}", err)
            },
        }
    }
}

#[derive(Debug)]
pub enum CompactError {
    SinkError(SinkError),
    IO(tokio::io::Error),
    SerializationError(SerializationError),
}

impl From<tokio::io::Error>
for CompactError {
    fn from(err: tokio::io::Error) -> CompactError {
        CompactError::IO(err)
    }
}

impl From<SinkError>
for CompactError {
    fn from(err: SinkError) -> CompactError {
        CompactError::SinkError(err)
    }
}

impl From<SerializationError>
for CompactError {
    fn from(err: SerializationError) -> CompactError {
        CompactError::SerializationError(err)
    }
}

#[derive(Debug)]
pub enum SinkError {
    MissingPublicKey(Hash),
    InvalidSignature {
        hash: Hash,
        err: Option<pqcrypto_traits::Error>,
    }
}

impl std::fmt::Display
for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SinkError::MissingPublicKey(hash) => {
                write!(f, "The public key ({}) for signature could not be found in the chain-of-trust", hash.to_string())
            },
            SinkError::InvalidSignature { hash, err } => {
                match err {
                    Some(err) => write!(f, "Failed verification of hash while using public key ({}) - {}", hash.to_string(), err),
                    None => write!(f, "Failed verification of hash while using public key ({})", hash.to_string())
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum SerializationError
{
    NoPrimarykey,
    NoData,
    EncodeError(RmpEncodeError),
    DecodeError(RmpDecodeError),
    JsonError(JsonError),
}

impl From<RmpEncodeError>
for SerializationError {
    fn from(err: RmpEncodeError) -> SerializationError {
        SerializationError::EncodeError(err)
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
            SerializationError::EncodeError(err) => {
                write!(f, "MessagePack encoding error - {}", err)
            },
            SerializationError::DecodeError(err) => {
                write!(f, "MessagePack decoding error - {}", err)
            },
            SerializationError::JsonError(err) => {
                write!(f, "JSON serialization error - {}", err)
            },
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    NotFound,
    Locked,
    AlreadyDeleted,
    InternalError(String),
    Tombstoned,
    SerializationError(SerializationError),
    TransformationError(TransformError),
    IO(tokio::io::Error),
}

impl From<tokio::io::Error>
for LoadError
{
    fn from(err: tokio::io::Error) -> LoadError {
        LoadError::IO(err)
    }   
}

impl From<SerializationError>
for LoadError
{
    fn from(err: SerializationError) -> LoadError {
        LoadError::SerializationError(err)
    }   
}

impl From<TransformError>
for LoadError
{
    fn from(err: TransformError) -> LoadError {
        LoadError::TransformationError(err)
    }   
}

#[derive(Debug)]
pub enum FeedError {
    SinkError(SinkError),
    IO(tokio::io::Error),
    ValidationError(ValidationError),
    SerializationError(SerializationError),
}

impl From<SinkError>
for FeedError
{
    fn from(err: SinkError) -> FeedError {
        FeedError::SinkError(err)
    }   
}

impl From<ValidationError>
for FeedError
{
    fn from(err: ValidationError) -> FeedError {
        FeedError::ValidationError(err)
    }   
}

impl From<tokio::io::Error>
for FeedError
{
    fn from(err: tokio::io::Error) -> FeedError {
        FeedError::IO(err)
    }   
}

impl From<SerializationError>
for FeedError
{
    fn from(err: SerializationError) -> FeedError {
        FeedError::SerializationError(err)
    }   
}

impl std::fmt::Display
for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FeedError::SinkError(err) => {
                write!(f, "Event sink error while processing a stream of events- {}", err)
            },
            FeedError::IO(err) => {
                write!(f, "IO sink error while processing a stream of events- {}", err)
            },
            FeedError::ValidationError(err) => {
                write!(f, "Validation error while processing a stream of events- {}", err)
            },
            FeedError::SerializationError(err) => {
                write!(f, "Serialization error while processing a stream of events- {}", err)
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct ProcessError
{
    pub sink_errors: Vec<SinkError>,
    pub validation_errors: Vec<ValidationError>,
}

impl ProcessError {
    pub fn has_errors(&self) -> bool {
        if self.sink_errors.is_empty() == false { return true; }
        if self.validation_errors.is_empty() == false { return true; }
        false
    }

    pub fn as_result(self) -> Result<(), ProcessError> {
        match self.has_errors() {
            true => Err(self),
            false => Ok(())
        }
    }
}

#[derive(Debug)]
pub enum ChainCreationError {
    ProcessError(ProcessError),
    IO(tokio::io::Error),
    SerializationError(SerializationError),
}

impl From<ProcessError>
for ChainCreationError
{
    fn from(err: ProcessError) -> ChainCreationError {
        ChainCreationError::ProcessError(err)
    }   
}

impl From<SerializationError>
for ChainCreationError
{
    fn from(err: SerializationError) -> ChainCreationError {
        ChainCreationError::SerializationError(err)
    }   
}

impl From<tokio::io::Error>
for ChainCreationError
{
    fn from(err: tokio::io::Error) -> ChainCreationError {
        ChainCreationError::IO(err)
    }   
}

#[derive(Debug)]
pub enum LintError {
    IO(std::io::Error),
    MissingWriteKey(Hash),
    NoAuthorization,
}

impl From<std::io::Error>
for LintError
{
    fn from(err: std::io::Error) -> LintError {
        LintError::IO(err)
    }   
}

impl std::fmt::Display
for LintError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LintError::IO(err) => {
                write!(f, "IO error while linting an event - {}", err)
            },
            LintError::MissingWriteKey(hash) => {
                write!(f, "Could not find the write public key ({}) in the seesion", hash.to_string())
            },
            LintError::NoAuthorization => {
                write!(f, "Event has no authorization metadata")
            },
        }
    }
}

#[derive(Debug)]
pub enum ValidationError {
    AllAbstained,
    Detached,
}

impl std::fmt::Display
for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationError::AllAbstained => {
                write!(f, "None of the validators approved this event")
            },
            ValidationError::Detached => {
                write!(f, "The event is detached from the chain of trust")
            },
        }
    }
}