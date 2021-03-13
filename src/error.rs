use super::crypto::Hash;
use super::header::PrimaryKey;

extern crate rmp_serde as rmps;

use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;
use serde_json::Error as JsonError;
use tokio::task::JoinError;
use std::time::SystemTimeError;
use std::sync::mpsc as smpsc;
use tokio::sync::mpsc as mpsc;

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
    MissingReadKey(Hash),
    UnspecifiedReadability,
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
            TransformError::MissingReadKey(key) => {
                write!(f, "Missing the read key ({}) needed to encrypt/decrypt this data object", key.to_string())
            },
            TransformError::UnspecifiedReadability => {
                write!(f, "The readability for this data object has not been specified")
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
    #[allow(dead_code)]
    CollectionDetached,
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
            SerializationError::CollectionDetached => {
                write!(f, "Collection is detached from a parent")
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
    SerializationError(SerializationError),
    TransformationError(TransformError),
    IO(tokio::io::Error),
    #[allow(dead_code)]
    CollectionDetached,
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
            LoadError::SerializationError(err) => {
                write!(f, "Serialization error while attempting to load data object - {}", err)
            },
            LoadError::TransformationError(err) => {
                write!(f, "Transformation error while attempting to load data object - {}", err)
            },
            LoadError::IO(err) => {
                write!(f, "IO error while attempting to load data object - {}", err)
            },
            LoadError::CollectionDetached => {
                write!(f, "Collection is detached from its parent, it must be attached before it can be used")
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

impl std::fmt::Display
for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut err = "Processing error - ".to_string();
        for sink in self.sink_errors.iter() {
            err = err + &sink.to_string()[..] + " - ";
        }
        for validation in self.validation_errors.iter() {
            err = err + &validation.to_string()[..] + " - ";
        }
        write!(f, "{}", err)
    }
}

#[derive(Debug)]
pub enum ChainCreationError {
    ProcessError(ProcessError),
    IO(tokio::io::Error),
    SerializationError(SerializationError),
    NoRootFound,
    #[allow(dead_code)]
    NotThisRoot,
    #[allow(dead_code)]
    NotImplemented,
    CommsError(CommsError),
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

impl From<CommsError>
for ChainCreationError
{
    fn from(err: CommsError) -> ChainCreationError {
        ChainCreationError::CommsError(err)
    }   
}

impl std::fmt::Display
for ChainCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChainCreationError::ProcessError(err) => {
                write!(f, "Failed to create chain-of-trust due to a processingerror - {}", err)
            },
            ChainCreationError::SerializationError(err) => {
                write!(f, "Failed to create chain-of-trust due to a serialization error - {}", err)
            },
            ChainCreationError::IO(err) => {
                write!(f, "Failed to create chain-of-trust due to an IO error - {}", err)
            },
            ChainCreationError::NotImplemented => {
                write!(f, "Failed to create chain-of-trust as the method is not implemented")
            },
            ChainCreationError::NoRootFound => {
                write!(f, "Failed to create chain-of-trust as the root node is not found")
            },
            ChainCreationError::NotThisRoot => {
                write!(f, "Failed to create chain-of-trust as this is the wrong root node")
            },
            ChainCreationError::CommsError(err) => {
                write!(f, "Failed to create chain-of-trust due to a communication error - {}", err)
            },
        }
    }
}

#[derive(Debug)]
pub enum LintError {
    IO(std::io::Error),
    MissingWriteKey(Hash),
    NoAuthorization(PrimaryKey),
    NoAuthorizationOrphan,
    SerializationError(SerializationError),
    TimeError(TimeError),
    UnspecifiedWritability,
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
            LintError::UnspecifiedWritability => {
                write!(f, "The writability of this data object has not been specified")
            },
        }
    }
}

#[derive(Debug)]
pub enum ValidationError {
    AllAbstained,
    Detached,
    NoSignatures,
    Time(TimeError),
}

impl From<TimeError>
for ValidationError
{
    fn from(err: TimeError) -> ValidationError {
        ValidationError::Time(err)
    }   
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
            ValidationError::Time(err) => {
                write!(f, "The data object event has an issue with time - {}", err)
            },
        }
    }
}

pub enum TimeError
{
    IO(std::io::Error),
    SystemTimeError(SystemTimeError),
    BeyondTolerance(u32),
    NoTimestamp,
    OutOfBounds(std::time::Duration),
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
            TimeError::NoTimestamp => {
                write!(f, "The data object has no timestamp metadata attached to it")
            },
            TimeError::OutOfBounds(dur) => {
                let time = std::time::UNIX_EPOCH + *dur;
                let datetime: chrono::DateTime<chrono::Utc> = time.into();
                write!(f, "The network latency is beyond tolerance to synchronize the clocks - {}", datetime.format("%d/%m/%Y %T"))
            },
        }
    }
}

impl std::fmt::Debug
for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug)]
pub enum CommitError
{
    #[allow(dead_code)]
    Aborted,
    TransformError(TransformError),
    LintError(LintError),
    SinkError(SinkError),
    IO(tokio::io::Error),
    ValidationError(ValidationError),
    SerializationError(SerializationError),
    PipeError(String),
    RootError(String),
}

impl From<TransformError>
for CommitError
{
    fn from(err: TransformError) -> CommitError {
        CommitError::TransformError(err)
    }   
}

impl From<LintError>
for CommitError
{
    fn from(err: LintError) -> CommitError {
        CommitError::LintError(err)
    }   
}

impl From<SinkError>
for CommitError
{
    fn from(err: SinkError) -> CommitError {
        CommitError::SinkError(err)
    }   
}

impl From<ValidationError>
for CommitError
{
    fn from(err: ValidationError) -> CommitError {
        CommitError::ValidationError(err)
    }   
}

impl From<tokio::io::Error>
for CommitError
{
    fn from(err: tokio::io::Error) -> CommitError {
        CommitError::IO(err)
    }   
}

impl From<SerializationError>
for CommitError
{
    fn from(err: SerializationError) -> CommitError {
        CommitError::SerializationError(err)
    }   
}

impl From<smpsc::RecvError>
for CommitError
{
    fn from(err: smpsc::RecvError) -> CommitError {
        CommitError::PipeError(err.to_string())
    }   
}

impl<T> From<smpsc::SendError<T>>
for CommitError
{
    fn from(err: smpsc::SendError<T>) -> CommitError {
        CommitError::PipeError(err.to_string())
    }   
}

impl From<mpsc::error::RecvError>
for CommitError
{
    fn from(err: mpsc::error::RecvError) -> CommitError {
        CommitError::PipeError(err.to_string())
    }   
}

impl<T> From<mpsc::error::SendError<T>>
for CommitError
{
    fn from(err: mpsc::error::SendError<T>) -> CommitError {
        CommitError::PipeError(err.to_string())
    }   
}

impl std::fmt::Display
for CommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommitError::Aborted => {
                write!(f, "The transaction aborted before it could be completed")
            },
            CommitError::TransformError(err) => {
                write!(f, "Failed to commit the data due to an error transforming the data object into events - {}", err.to_string())
            },
            CommitError::LintError(err) => {
                write!(f, "Failed to commit the data due to an error linting the data object events - {}", err.to_string())
            },
            CommitError::SinkError(err) => {
                write!(f, "Failed to commit the data due to an error accepting the event into a sink - {}", err.to_string())
            },
            CommitError::IO(err) => {
                write!(f, "Failed to commit the data due to an IO error - {}", err.to_string())
            },
            CommitError::ValidationError(err) => {
                write!(f, "Failed to commit the data due to a validation error - {}", err.to_string())
            },
            CommitError::SerializationError(err) => {
                write!(f, "Failed to commit the data due to an serialization error - {}", err.to_string())
            },
            CommitError::PipeError(err) => {
                write!(f, "Failed to commit the data due to an error receiving the result in the interprocess pipe - {}", err.to_string())
            },
            CommitError::RootError(err) => {
                write!(f, "Failed to commit the data due to an error at the root server while processing the events - {}", err.to_string())
            },
        }
    }
}

pub enum CommsError
{
    EncodeError(RmpEncodeError),
    DecodeError(RmpDecodeError),
    SendError(String),
    ReceiveError(String),
    IO(std::io::Error),
    NoReplyChannel,
    Disconnected,
    #[allow(dead_code)]
    JoinError(JoinError),
    LoadError(LoadError),
    RootServerError(String),
    InternalError(String),
}

impl From<RmpEncodeError>
for CommsError
{
    fn from(err: RmpEncodeError) -> CommsError {
        CommsError::EncodeError(err)
    }   
}

impl From<RmpDecodeError>
for CommsError
{
    fn from(err: RmpDecodeError) -> CommsError {
        CommsError::DecodeError(err)
    }   
}

impl From<std::io::Error>
for CommsError
{
    fn from(err: std::io::Error) -> CommsError {
        CommsError::IO(err)
    }   
}

impl<T> From<mpsc::error::SendError<T>>
for CommsError
{
    fn from(err: mpsc::error::SendError<T>) -> CommsError {
        CommsError::SendError(err.to_string())
    }   
}

impl From<mpsc::error::RecvError>
for CommsError
{
    fn from(err: mpsc::error::RecvError) -> CommsError {
        CommsError::ReceiveError(err.to_string())
    }   
}

impl<T> From<smpsc::SendError<T>>
for CommsError
{
    fn from(err: smpsc::SendError<T>) -> CommsError {
        CommsError::SendError(err.to_string())
    }   
}

impl From<smpsc::RecvError>
for CommsError
{
    fn from(err: smpsc::RecvError) -> CommsError {
        CommsError::ReceiveError(err.to_string())
    }   
}

impl<T> From<tokio::sync::broadcast::error::SendError<T>>
for CommsError
{
    fn from(err: tokio::sync::broadcast::error::SendError<T>) -> CommsError {
        CommsError::SendError(err.to_string())
    }   
}

impl From<tokio::sync::broadcast::error::RecvError>
for CommsError
{
    fn from(err: tokio::sync::broadcast::error::RecvError) -> CommsError {
        CommsError::ReceiveError(err.to_string())
    }   
}

impl From<JoinError>
for CommsError
{
    fn from(err: JoinError) -> CommsError {
        CommsError::ReceiveError(err.to_string())
    }   
}

impl From<LoadError>
for CommsError
{
    fn from(err: LoadError) -> CommsError {
        CommsError::LoadError(err)
    }   
}

impl From<ChainCreationError>
for CommsError
{
    fn from(err: ChainCreationError) -> CommsError {
        CommsError::RootServerError(err.to_string())
    }   
}

impl From<CommitError>
for CommsError
{
    fn from(err: CommitError) -> CommsError {
        CommsError::InternalError(err.to_string())
    }   
}

impl std::fmt::Display
for CommsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommsError::EncodeError(err) => {
                write!(f, "Encoding error while processing communication - {}", err)
            },
            CommsError::DecodeError(err) => {
                write!(f, "Decoding error while processing communication - {}", err)
            },
            CommsError::IO(err) => {
                write!(f, "IO error while processing communication - {}", err)
            },
            CommsError::SendError(err) => {
                write!(f, "Sending error while processing communication - {}", err)
            },
            CommsError::ReceiveError(err) => {
                write!(f, "Receiving error while processing communication - {}", err)
            },
            CommsError::NoReplyChannel => {
                write!(f, "Message has no reply channel attached to it")
            },
            CommsError::Disconnected => {
                write!(f, "Channel has been disconnected")
            },
            CommsError::JoinError(err) => {
                write!(f, "Receiving error while processing communication - {}", err)
            },
            CommsError::LoadError(err) => {
                write!(f, "Load error occured while processing communication - {}", err)
            },
            CommsError::RootServerError(err) => {
                write!(f, "Error at the root server while processing communication - {}", err)
            },
            CommsError::InternalError(err) => {
                write!(f, "Internal error while processing communication - {}", err)
            },
        }
    }
}

impl std::fmt::Debug
for CommsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[allow(dead_code)]
pub enum BusError
{
    NotImplemented,
}