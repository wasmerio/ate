use crate::redo::LogFilePointer;

use super::crypto::Hash;
use super::header::PrimaryKey;

extern crate rmp_serde as rmps;

use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;
use serde_json::Error as JsonError;
use std::time::SystemTimeError;

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
    NotFound(PrimaryKey),
    ObjectStillLocked(PrimaryKey),
    AlreadyDeleted(PrimaryKey),
    Tombstoned(PrimaryKey),
    MissingLogFileData(LogFilePointer),
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

impl std::fmt::Display
for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoadError::NotFound(key) => {
                write!(f, "Data object with key ({}) could not be found", key.as_hex_string())
            },
            LoadError::ObjectStillLocked(key) => {
                write!(f, "Data object with key ({}) is still being edited in the current scope", key.as_hex_string())
            },
            LoadError::AlreadyDeleted(key) => {
                write!(f, "Data object with key ({}) has already been deleted", key.as_hex_string())
            },
            LoadError::Tombstoned(key) => {
                write!(f, "Data object with key ({}) has already been tombstoned", key.as_hex_string())
            },
            LoadError::MissingLogFileData(pointer) => {
                write!(f, "Data object could not be found in the log file of version ({}) at ({})", pointer.version, pointer.offset)
            },
            LoadError::SerializationError(err) => {
                write!(f, "Serialization error while attempting to load data object - {}", err)
            },
            LoadError::TransformationError(err) => {
                write!(f, "Transformation error while attempting to load data object - {}", err)
            },
            LoadError::IO(err) => {
                write!(f, "IO error while attempting to load data object - {}", err)
            },
        }
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
                write!(f, "Event sink error while processing a stream of events - {}", err)
            },
            FeedError::IO(err) => {
                write!(f, "IO sink error while processing a stream of events - {}", err)
            },
            FeedError::ValidationError(err) => {
                write!(f, "Validation error while processing a stream of events - {}", err)
            },
            FeedError::SerializationError(err) => {
                write!(f, "Serialization error while processing a stream of events - {}", err)
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
    MissingAuthorizationMetadata(PrimaryKey),
    MissingAuthorizationMetadataOrphan,
    NoAuthorization(PrimaryKey),
    NoAuthorizationOrphan,
    SerializationError(SerializationError),
    TimeError(TimeError),
}

impl From<std::io::Error>
for LintError
{
    fn from(err: std::io::Error) -> LintError {
        LintError::IO(err)
    }   
}

impl From<SerializationError>
for LintError
{
    fn from(err: SerializationError) -> LintError {
        LintError::SerializationError(err)
    }   
}

impl From<TimeError>
for LintError
{
    fn from(err: TimeError) -> LintError {
        LintError::TimeError(err)
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
                write!(f, "Could not find the write public key ({}) in the session", hash.to_string())
            },
            LintError::MissingAuthorizationMetadata(key) => {
                write!(f, "Data object with key ({}) has no write authorization metadata attached to it", key.as_hex_string())
            },
            LintError::MissingAuthorizationMetadataOrphan => {
                write!(f, "Data object without a primary has no write authorization metadata attached to it")
            },
            LintError::NoAuthorization(key) => {
                write!(f, "Data object with key ({}) has no write authorization in its metadata", key.as_hex_string())
            },
            LintError::NoAuthorizationOrphan => {
                write!(f, "Data objects without a primary key has no write authorization")
            },
            LintError::SerializationError(err) => {
                write!(f, "Serialization error while linting data object - {}", err)
            },
            LintError::TimeError(err) => {
                write!(f, "Timing error while linting data object - {}", err)
            },
        }
    }
}

#[derive(Debug)]
pub enum ValidationError {
    AllAbstained,
    Detached,
    NoSignatures,
}

impl std::fmt::Display
for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationError::AllAbstained => {
                write!(f, "None of the validators approved this data object event")
            },
            ValidationError::Detached => {
                write!(f, "The data object event is detached from the chain of trust")
            },
            ValidationError::NoSignatures => {
                write!(f, "The data object event has no signatures")
            },
        }
    }
}

#[derive(Debug)]
pub enum FlushError {
    FeedError(FeedError),
    TransformError(TransformError),
    LintError(LintError),
    SerializationError(SerializationError),
}

impl From<FeedError>
for FlushError
{
    fn from(err: FeedError) -> FlushError {
        FlushError::FeedError(err)
    }   
}

impl From<TransformError>
for FlushError
{
    fn from(err: TransformError) -> FlushError {
        FlushError::TransformError(err)
    }   
}

impl From<LintError>
for FlushError
{
    fn from(err: LintError) -> FlushError {
        FlushError::LintError(err)
    }   
}

impl From<SerializationError>
for FlushError
{
    fn from(err: SerializationError) -> FlushError {
        FlushError::SerializationError(err)
    }   
}

impl std::fmt::Display
for FlushError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FlushError::FeedError(err) => {
                write!(f, "Failed to flush the data due to an error feeding events into the chain of trust - {}", err.to_string())
            },
            FlushError::TransformError(err) => {
                write!(f, "Failed to flush the data due to an error transforming the data object into events - {}", err.to_string())
            },
            FlushError::LintError(err) => {
                write!(f, "Failed to flush the data due to an error linting the data object events - {}", err.to_string())
            },
            FlushError::SerializationError(err) => {
                write!(f, "Failed to flush the data due to an serialization error - {}", err.to_string())
            },
        }
    }
}

#[derive(Debug)]
pub enum TimeError
{
    IO(std::io::Error),
    SystemTimeError(SystemTimeError),
    BeyondTolerance(u32)
}

impl From<std::io::Error>
for TimeError
{
    fn from(err: std::io::Error) -> TimeError {
        TimeError::IO(err)
    }   
}

impl From<SystemTimeError>
for TimeError
{
    fn from(err: SystemTimeError) -> TimeError {
        TimeError::SystemTimeError(err)
    }   
}

impl std::fmt::Display
for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimeError::IO(err) => {
                write!(f, "IO error while computing the current time - {}", err.to_string())
            },
            TimeError::SystemTimeError(err) => {
                write!(f, "System clock error while computing the current time - {}", err.to_string())
            },
            TimeError::BeyondTolerance(err) => {
                write!(f, "The network latency is beyond tolerance to synchronize the clocks - {}", err.to_string())
            },
        }
    }
}