#![allow(unused_imports)]
use log::{info, error, debug};
use std::error::Error;
use super::crypto::Hash;
use super::header::PrimaryKey;
use serde::{Serialize, de::DeserializeOwned, Deserialize};

extern crate rmp_serde as rmps;

use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;
use serde_json::Error as JsonError;
use tokio::task::JoinError;
use std::time::SystemTimeError;
use std::sync::mpsc as smpsc;
use tokio::sync::mpsc as mpsc;
use tokio::sync::broadcast as broadcast;
use trust_dns_proto::error::ProtoError as DnsProtoError;
use trust_dns_client::error::ClientError as DnsClientError;

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

impl std::error::Error
for CryptoError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
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

impl std::error::Error
for TransformError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum CompactError {
    SinkError(SinkError),
    IO(tokio::io::Error),
    LoadError(LoadError),
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

impl From<LoadError>
for CompactError {
    fn from(err: LoadError) -> CompactError {
        CompactError::LoadError(err)
    }
}

impl From<SerializationError>
for CompactError {
    fn from(err: SerializationError) -> CompactError {
        CompactError::SerializationError(err)
    }
}

impl std::fmt::Display
for CompactError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CompactError::IO(err) => {
                write!(f, "Failed to compact the chain due to an IO error - {}", err)
            },
            CompactError::SerializationError(err) => {
                write!(f, "Failed to compact the chain due to a serialization error - {}", err)
            },
            CompactError::SinkError(err) => {
                write!(f, "Failed to compact the chain due to an error in the sink - {}", err)
            },
            CompactError::LoadError(err) => {
                write!(f, "Failed to compact the chain due to an error loaded on event - {}", err)
            },
        }
    }
}

impl std::error::Error
for CompactError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum SinkError {
    MissingPublicKey(Hash),
    Trust(TrustError),
    InvalidSignature {
        hash: Hash,
        err: Option<pqcrypto_traits::Error>,
    }
}

impl From<TrustError>
for SinkError
{
    fn from(err: TrustError) -> SinkError {
        SinkError::Trust(err)
    }   
}

impl std::fmt::Display
for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SinkError::MissingPublicKey(hash) => {
                write!(f, "The public key ({}) for signature could not be found in the chain-of-trust", hash.to_string())
            },
            SinkError::Trust(err) => {
                write!(f, "Failed to accept event due to a trust error - {}", err)
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

impl std::error::Error
for SinkError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

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
        }
    }
}

impl std::error::Error
for SerializationError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum LoadError {
    NotFound(PrimaryKey),
    NoPrimaryKey,
    VersionMismatch,
    NotFoundByHash(Hash),
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

impl From<RmpEncodeError>
for LoadError {
    fn from(err: RmpEncodeError) -> LoadError {
        LoadError::SerializationError(SerializationError::EncodeError(err))
    }
}

impl From<RmpDecodeError>
for LoadError {
    fn from(err: RmpDecodeError) -> LoadError {
        LoadError::SerializationError(SerializationError::DecodeError(err))
    }
}

impl std::fmt::Display
for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoadError::NotFound(key) => {
                write!(f, "Data object with key ({}) could not be found", key.as_hex_string())
            },
            LoadError::NotFoundByHash(hash) => {
                write!(f, "Data object with hash ({}) could not be found", hash.to_string())
            },
            LoadError::VersionMismatch => {
                write!(f, "Entry has an invalid version for this log file")
            }
            LoadError::NoPrimaryKey => {
                write!(f, "Entry has no primary could and hence could not be loaded")
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

impl std::error::Error
for LoadError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
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

impl std::error::Error
for ProcessError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum ChainCreationError {
    ProcessError(ProcessError),
    IO(tokio::io::Error),
    SerializationError(SerializationError),
    NoRootFoundInConfig,
    NoRootFoundForUrl(String),
    UnsupportedProtocol,
    UrlInvalid(url::ParseError),
    NotSupported,
    #[allow(dead_code)]
    NotThisRoot,
    #[allow(dead_code)]
    NotImplemented,
    TimeError(TimeError),
    NoValidDomain(String),
    CommsError(CommsError),
    DnsProtoError(DnsProtoError),
    DnsClientError(DnsClientError),
    ServerRejected(String),
    InternalError(String),
}

impl From<url::ParseError>
for ChainCreationError
{
    fn from(err: url::ParseError) -> ChainCreationError {
        ChainCreationError::UrlInvalid(err)
    }   
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

impl From<DnsProtoError>
for ChainCreationError
{
    fn from(err: DnsProtoError) -> ChainCreationError {
        ChainCreationError::DnsProtoError(err)
    }
}

impl From<DnsClientError>
for ChainCreationError
{
    fn from(err: DnsClientError) -> ChainCreationError {
        ChainCreationError::DnsClientError(err)
    }
}

impl From<TimeError>
for ChainCreationError
{
    fn from(err: TimeError) -> ChainCreationError {
        ChainCreationError::TimeError(err)
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
            ChainCreationError::UrlInvalid(err) => {
                write!(f, "Failed to create chain-of-trust due to a parsing the chain URL - {}", err)
            },
            ChainCreationError::IO(err) => {
                write!(f, "Failed to create chain-of-trust due to an IO error - {}", err)
            },
            ChainCreationError::NotImplemented => {
                write!(f, "Failed to create chain-of-trust as the method is not implemented")
            },
            ChainCreationError::NotSupported => {
                write!(f, "Failed to create chain-of-trust as the operation is not supported. Possible causes are calling 'open_by_key' on a Registry which only supports the 'open_by_url'.")
            },
            ChainCreationError::NoRootFoundInConfig => {
                write!(f, "Failed to create chain-of-trust as the root node is not found in the configuration settings")
            },
            ChainCreationError::NoRootFoundForUrl(url) => {
                write!(f, "Failed to create chain-of-trust as the root node is not found in the URL [{}]", url)
            },
            ChainCreationError::UnsupportedProtocol => {
                write!(f, "Failed to create chain-of-trust as the protocol is not supported (only TCP is supported)")
            },
            ChainCreationError::NotThisRoot => {
                write!(f, "Failed to create chain-of-trust as this is the wrong root node")
            },
            ChainCreationError::CommsError(err) => {
                write!(f, "Failed to create chain-of-trust due to a communication error - {}", err)
            },
            ChainCreationError::NoValidDomain(err) => {
                write!(f, "Failed to create chain-of-trust as the address does not have a valid domain name [{}]", err)
            },
            ChainCreationError::DnsProtoError(err) => {
                write!(f, "Failed to create chain-of-trust due to a DNS error - {}", err)
            },
            ChainCreationError::DnsClientError(err) => {
                write!(f, "Failed to create chain-of-trust due to a DNS error - {}", err)
            },
            ChainCreationError::ServerRejected(reason) => {
                write!(f, "Failed to create chain-of-trust as the server refused to create the chain ({})", reason)
            },
            ChainCreationError::TimeError(err) => {
                write!(f, "Failed to create chain-of-trust due error with time keeping - {}", err)
            },
            ChainCreationError::InternalError(err) => {
                write!(f, "{}", err)
            },
        }
    }
}

impl std::error::Error
for ChainCreationError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum TrustError
{
    NoAuthorization(PrimaryKey),
    NoAuthorizationOrphan,
    MissingParent(PrimaryKey),
    Time(TimeError),
    UnspecifiedWritability,
}

impl From<TimeError>
for TrustError
{
    fn from(err: TimeError) -> TrustError {
        TrustError::Time(err)
    }   
}

impl std::fmt::Display
for TrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TrustError::NoAuthorization(key) => {
                write!(f, "Data object with key ({}) has no write authorization in its metadata", key.as_hex_string())
            },
            TrustError::MissingParent(key) => {
                write!(f, "Data object references a parent object that does not exist ({})", key.as_hex_string())
            },
            TrustError::NoAuthorizationOrphan => {
                write!(f, "Data objects without a primary key has no write authorization")
            },
            TrustError::Time(err) => {
                write!(f, "Timing error while linting data object - {}", err)
            },
            TrustError::UnspecifiedWritability => {
                write!(f, "The writability of this data object has not been specified")
            },
        }
    }
}

impl std::error::Error
for TrustError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum LintError {
    IO(std::io::Error),
    MissingWriteKey(Hash),
    Trust(TrustError),
    SerializationError(SerializationError),
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

impl From<TrustError>
for LintError
{
    fn from(err: TrustError) -> LintError {
        LintError::Trust(err)
    }   
}

impl From<TimeError>
for LintError
{
    fn from(err: TimeError) -> LintError {
        LintError::Trust(TrustError::Time(err))
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
            LintError::SerializationError(err) => {
                write!(f, "Serialization error while linting data object - {}", err)
            },
            LintError::Trust(err) => {
                write!(f, "Trust error while linting data object - {}", err)
            },
        }
    }
}

impl std::error::Error
for LintError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum ValidationError {
    Denied,
    AllAbstained,
    Detached,
    NoSignatures,
    Trust(TrustError),
}

impl From<TrustError>
for ValidationError
{
    fn from(err: TrustError) -> ValidationError {
        ValidationError::Trust(err)
    }   
}

impl From<TimeError>
for ValidationError
{
    fn from(err: TimeError) -> ValidationError {
        ValidationError::Trust(TrustError::Time(err))
    }   
}

impl std::fmt::Display
for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationError::AllAbstained => {
                write!(f, "None of the validators approved this data object event")
            },
            ValidationError::Denied => {
                write!(f, "The data was rejected by one of the validators")
            },
            ValidationError::Detached => {
                write!(f, "The data object event is detached from the chain of trust")
            },
            ValidationError::NoSignatures => {
                write!(f, "The data object event has no signatures and one is required to store it at this specific location within the chain of trust")
            },
            ValidationError::Trust(err) => {
                write!(f, "The data object event has an issue with trust - {}", err)
            },
        }
    }
}

impl std::error::Error
for ValidationError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
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

impl std::error::Error
for TimeError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum CommitError
{
    #[allow(dead_code)]
    Aborted,
    NewRootsAreDisabled,
    TransformError(TransformError),
    LintError(LintError),
    SinkError(SinkError),
    IO(tokio::io::Error),
    ValidationError(Vec<ValidationError>),
    SerializationError(SerializationError),
    PipeError(String),
    RootError(String),
    CommsError(CommsError),
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

impl From<CommsError>
for CommitError
{
    fn from(err: CommsError) -> CommitError {
        CommitError::CommsError(err)
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
        let mut errors = Vec::new();
        errors.push(err);
        CommitError::ValidationError(errors)
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

impl<T> From<broadcast::error::SendError<T>>
for CommitError
{
    fn from(err: broadcast::error::SendError<T>) -> CommitError {
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
            CommitError::NewRootsAreDisabled => {
                write!(f, "New root objects are currently not allowed for this chain")
            },
            CommitError::TransformError(err) => {
                write!(f, "Failed to commit the data due to an error transforming the data object into events - {}", err.to_string())
            },
            CommitError::CommsError(err) => {
                write!(f, "Failed to commit the data due to an error in communication - {}", err.to_string())
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
            CommitError::ValidationError(errs) => {
                write!(f, "Failed to commit the data due to a validation error")?;
                for err in errs.iter() {
                    write!(f, " - {}", err.to_string())?;
                }
                Ok(())
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

impl std::error::Error
for CommitError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum CommsError
{
    SerializationError(SerializationError),
    SendError(String),
    ReceiveError(String),
    IO(std::io::Error),
    NoReplyChannel,
    NoWireFormat,
    Disconnected,
    ShouldBlock,
    ValidationError(Vec<ValidationError>),
    #[allow(dead_code)]
    JoinError(JoinError),
    LoadError(LoadError),
    RootServerError(String),
    InternalError(String),
}

impl From<SerializationError>
for CommsError
{
    fn from(err: SerializationError) -> CommsError {
        CommsError::SerializationError(err)
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

impl From<smpsc::RecvError>
for CommsError
{
    fn from(err: smpsc::RecvError) -> CommsError {
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
        match err {
            CommitError::ValidationError(errs) => CommsError::ValidationError(errs),
            err => CommsError::InternalError(format!("commit-failed - {}", err.to_string())),
        }
    }   
}

impl From<bincode::Error>
for CommsError
{
    fn from(err: bincode::Error) -> CommsError {
        CommsError::SerializationError(SerializationError::BincodeError(err))
    }   
}

impl From<RmpDecodeError>
for CommsError {
    fn from(err: RmpDecodeError) -> CommsError {
        CommsError::SerializationError(SerializationError::DecodeError(err))
    }
}

impl From<JsonError>
for CommsError {
    fn from(err: JsonError) -> CommsError {
        CommsError::SerializationError(SerializationError::JsonError(err))
    }
}

impl std::fmt::Display
for CommsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommsError::SerializationError(err) => {
                write!(f, "Serialization error while processing communication - {}", err)
            },
            CommsError::IO(err) => {
                write!(f, "IO error while processing communication - {}", err)
            },
            CommsError::ShouldBlock => {
                write!(f, "Operation should have blocked but it didn't")
            }
            CommsError::SendError(err) => {
                write!(f, "Sending error while processing communication - {}", err)
            },
            CommsError::ReceiveError(err) => {
                write!(f, "Receiving error while processing communication - {}", err)
            },
            CommsError::NoReplyChannel => {
                write!(f, "Message has no reply channel attached to it")
            },
            CommsError::NoWireFormat => {
                write!(f, "Server did not send a wire format")
            },
            CommsError::ValidationError(errs) => {
                write!(f, "Message contained event data that failed validation")?;
                for err in errs.iter() {
                    write!(f, " - {}", err.to_string())?;
                }
                Ok(())
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
                write!(f, "Internal comms error - {}", err)
            },
        }
    }
}

impl std::error::Error
for CommsError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum BusError
{
    LoadError(LoadError),
    ReceiveError(String),
    ChannelClosed,
    SerializationError(SerializationError),
    LockError(LockError),
    TransformError(TransformError),
}

impl From<LoadError>
for BusError
{
    fn from(err: LoadError) -> BusError {
        BusError::LoadError(err)
    }   
}

impl From<TransformError>
for BusError
{
    fn from(err: TransformError) -> BusError {
        BusError::TransformError(err)
    }   
}

impl From<SerializationError>
for BusError
{
    fn from(err: SerializationError) -> BusError {
        BusError::SerializationError(err)
    }   
}

impl From<mpsc::error::RecvError>
for BusError
{
    fn from(err: mpsc::error::RecvError) -> BusError {
        BusError::ReceiveError(err.to_string())
    }   
}

impl From<smpsc::RecvError>
for BusError
{
    fn from(err: smpsc::RecvError) -> BusError {
        BusError::ReceiveError(err.to_string())
    }   
}

impl From<LockError>
for BusError
{
    fn from(err: LockError) -> BusError {
        BusError::LockError(err)
    }   
}

impl std::fmt::Display
for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BusError::LoadError(err) => {
                write!(f, "Failed to receive event from BUS due to an error loading the event - {}", err)
            },
            BusError::TransformError(err) => {
                write!(f, "Failed to receive event from BUS due to an error transforming the data - {}", err)
            },
            BusError::ReceiveError(err) => {
                write!(f, "Failed to receive event from BUS due to an internal error  - {}", err)
            },
            BusError::ChannelClosed => {
                write!(f, "Failed to receive event from BUS as the channel is closed")
            },
            BusError::SerializationError(err) => {
                write!(f, "Failed to send event to the BUS due to an error in serialization - {}", err)
            },
            BusError::LockError(err) => {
                write!(f, "Failed to receive event from BUS due to an error locking the data object - {}", err)
            },
        }
    }
}

impl std::error::Error
for BusError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum LockError
{
    SerializationError(SerializationError),
    LintError(LintError),
    CommitError(String),
    ReceiveError(String),
}

impl From<SerializationError>
for LockError
{
    fn from(err: SerializationError) -> LockError {
        LockError::SerializationError(err)
    }   
}

impl From<LintError>
for LockError
{
    fn from(err: LintError) -> LockError {
        LockError::LintError(err)
    }   
}

impl From<CommitError>
for LockError
{
    fn from(err: CommitError) -> LockError {
        LockError::CommitError(err.to_string())
    }   
}

impl From<mpsc::error::RecvError>
for LockError
{
    fn from(err: mpsc::error::RecvError) -> LockError {
        LockError::ReceiveError(err.to_string())
    }   
}

impl From<smpsc::RecvError>
for LockError
{
    fn from(err: smpsc::RecvError) -> LockError {
        LockError::ReceiveError(err.to_string())
    }   
}

impl std::fmt::Display
for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LockError::SerializationError(err) => {
                write!(f, "Failed to lock the data object due to a serialization error - {}", err)
            },
            LockError::LintError(err) => {
                write!(f, "Failed to lock the data object due to issue linting the event - {}", err)
            },
            LockError::CommitError(err) => {
                write!(f, "Failed to lock the data object due to issue committing the event to the pipe - {}", err)
            },
            LockError::ReceiveError(err) => {
                write!(f, "Failed to lock the data object due to an error receiving on the pipe - {}", err)
            },
        }
    }
}

impl std::error::Error
for LockError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum InvokeError<E>
{
    Reply(E),
    LoadError(LoadError),
    SerializationError(SerializationError),
    CommitError(CommitError),
    LockError(LockError),
    PipeError(String),
    ServiceError(String),
    Timeout,
    Aborted
}

impl<E> From<SerializationError>
for InvokeError<E>
{
    fn from(err: SerializationError) -> InvokeError<E> {
        InvokeError::SerializationError(err)
    }   
}

impl<E> From<LockError>
for InvokeError<E>
{
    fn from(err: LockError) -> InvokeError<E> {
        InvokeError::LockError(err)
    }   
}

impl<T, E> From<mpsc::error::SendError<T>>
for InvokeError<E>
{
    fn from(err: mpsc::error::SendError<T>) -> InvokeError<E> {
        InvokeError::PipeError(err.to_string())
    }   
}

impl<E> From<LoadError>
for InvokeError<E>
{
    fn from(err: LoadError) -> InvokeError<E> {
        InvokeError::LoadError(err)
    }   
}

impl<E> From<CommitError>
for InvokeError<E>
{
    fn from(err: CommitError) -> InvokeError<E> {
        InvokeError::CommitError(err)
    }   
}

impl<E> From<tokio::time::error::Elapsed>
for InvokeError<E>
{
    fn from(_elapsed: tokio::time::error::Elapsed) -> InvokeError<E> {
        InvokeError::Timeout
    }
}

impl<E> std::fmt::Display
for InvokeError<E>
where E: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InvokeError::LoadError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::SerializationError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::LockError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::CommitError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::PipeError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::Reply(_) => {
                write!(f, "Command failed for an unspecified reason")
            },
            InvokeError::ServiceError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::Timeout => {
                write!(f, "Command failed - Timeout")
            },
            InvokeError::Aborted => {
                write!(f, "Command failed - Aborted")
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServiceErrorReply<E>
{
    Reply(E),
    ServiceError(String),
}

#[derive(Debug)]
pub enum ServiceError<E>
{
    Reply(E),
    IO(tokio::io::Error),
    LoadError(LoadError),
    SerializationError(SerializationError),
    ChainCreationError(ChainCreationError),
    CommitError(CommitError),
    LockError(LockError),
    PipeError(String),
    ServiceError(String),
    Timeout,
    Aborted
}

impl<E> From<SerializationError>
for ServiceError<E>
{
    fn from(err: SerializationError) -> ServiceError<E> {
        ServiceError::SerializationError(err)
    }   
}

impl<E> From<LockError>
for ServiceError<E>
{
    fn from(err: LockError) -> ServiceError<E> {
        ServiceError::LockError(err)
    }   
}

impl<T, E> From<mpsc::error::SendError<T>>
for ServiceError<E>
{
    fn from(err: mpsc::error::SendError<T>) -> ServiceError<E> {
        ServiceError::PipeError(err.to_string())
    }   
}

impl<E> From<LoadError>
for ServiceError<E>
{
    fn from(err: LoadError) -> ServiceError<E> {
        ServiceError::LoadError(err)
    }   
}

impl<E> From<CommitError>
for ServiceError<E>
{
    fn from(err: CommitError) -> ServiceError<E> {
        ServiceError::CommitError(err)
    }   
}

impl<E> From<ChainCreationError>
for ServiceError<E>
{
    fn from(err: ChainCreationError) -> ServiceError<E> {
        ServiceError::ChainCreationError(err)
    }   
}

impl<E> From<tokio::time::error::Elapsed>
for ServiceError<E>
{
    fn from(_elapsed: tokio::time::error::Elapsed) -> ServiceError<E> {
        ServiceError::Timeout
    }
}

impl<E> From<tokio::io::Error>
for ServiceError<E>
{
    fn from(err: tokio::io::Error) -> ServiceError<E> {
        ServiceError::IO(err)
    }   
}

impl<E> From<ServiceErrorReply<E>>
for ServiceError<E>
{
    fn from(err: ServiceErrorReply<E>) -> ServiceError<E> {
        match err {
            ServiceErrorReply::Reply(err) => ServiceError::Reply(err),
            ServiceErrorReply::ServiceError(err) => ServiceError::ServiceError(err)
        }
    } 
}

impl<E> ServiceError<E>
{
    pub fn as_reply(self) -> (ServiceErrorReply<E>, ServiceError<()>)
    {
        match self {
            ServiceError::Reply(e) => (ServiceErrorReply::Reply(e), ServiceError::Reply(())),
            err => {
                let err = err.strip();
                (
                    ServiceErrorReply::ServiceError(err.to_string()),
                    err
                )
            }
        }
    }

    pub fn strip(self) -> ServiceError<()>
    {
        match self {
            ServiceError::LoadError(a) => ServiceError::LoadError(a),
            ServiceError::SerializationError(a) => ServiceError::SerializationError(a),
            ServiceError::LockError(a) => ServiceError::LockError(a),
            ServiceError::CommitError(a) => ServiceError::CommitError(a),
            ServiceError::ChainCreationError(a) => ServiceError::ChainCreationError(a),
            ServiceError::PipeError(a) => ServiceError::PipeError(a),
            ServiceError::IO(a) => ServiceError::IO(a),
            ServiceError::Reply(_) => ServiceError::Reply(()),
            ServiceError::ServiceError(a) => ServiceError::ServiceError(a),
            ServiceError::Timeout => ServiceError::Timeout,
            ServiceError::Aborted => ServiceError::Aborted,
        }
    }
}

impl<E> std::fmt::Display
for ServiceError<E>
where E: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ServiceError::LoadError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::SerializationError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::LockError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::CommitError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::ChainCreationError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::PipeError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::IO(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::ServiceError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::Reply(err) => {
                write!(f, "Command failed - {:?}", err)
            },
            ServiceError::Timeout => {
                write!(f, "Command failed - Timeout")
            },
            ServiceError::Aborted => {
                write!(f, "Command failed - Aborted")
            },
        }
    }
}

/// Super-set of all errors that could possibly happen within this ATE library
/// This error allows one to roll up all the errors into a clean object using
/// standard from conversions for cleaner code.
#[derive(Debug)]
pub enum AteError
{
    LockError(LockError),
    BusError(BusError),
    CommsError(CommsError),
    CommitError(CommitError),
    TimeError(TimeError),
    LintError(LintError),
    ChainCreationError(ChainCreationError),
    ProcessError(ProcessError),
    SerializationError(SerializationError),
    SinkError(SinkError),
    CompactError(CompactError),
    LoadError(LoadError),
    IO(tokio::io::Error),
    CryptoError(CryptoError),
    TransformError(TransformError),
    InvokeError(String),
    ServiceError(String),
    NotImplemented,
}

impl From<LockError>
for AteError
{
    fn from(err: LockError) -> AteError {
        AteError::LockError(err)
    }   
}

impl From<BusError>
for AteError
{
    fn from(err: BusError) -> AteError {
        AteError::BusError(err)
    }   
}

impl From<CommsError>
for AteError
{
    fn from(err: CommsError) -> AteError {
        AteError::CommsError(err)
    }   
}

impl From<CommitError>
for AteError
{
    fn from(err: CommitError) -> AteError {
        AteError::CommitError(err)
    }   
}

impl From<TimeError>
for AteError
{
    fn from(err: TimeError) -> AteError {
        AteError::TimeError(err)
    }   
}

impl From<LintError>
for AteError
{
    fn from(err: LintError) -> AteError {
        AteError::LintError(err)
    }   
}

impl From<ChainCreationError>
for AteError
{
    fn from(err: ChainCreationError) -> AteError {
        AteError::ChainCreationError(err)
    }   
}

impl From<ProcessError>
for AteError
{
    fn from(err: ProcessError) -> AteError {
        AteError::ProcessError(err)
    }   
}

impl From<SerializationError>
for AteError
{
    fn from(err: SerializationError) -> AteError {
        AteError::SerializationError(err)
    }   
}

impl From<SinkError>
for AteError
{
    fn from(err: SinkError) -> AteError {
        AteError::SinkError(err)
    }   
}

impl From<CompactError>
for AteError
{
    fn from(err: CompactError) -> AteError {
        AteError::CompactError(err)
    }   
}

impl From<LoadError>
for AteError
{
    fn from(err: LoadError) -> AteError {
        AteError::LoadError(err)
    }   
}

impl From<TransformError>
for AteError
{
    fn from(err: TransformError) -> AteError {
        AteError::TransformError(err)
    }   
}

impl From<CryptoError>
for AteError
{
    fn from(err: CryptoError) -> AteError {
        AteError::CryptoError(err)
    }   
}

impl From<tokio::io::Error>
for AteError
{
    fn from(err: tokio::io::Error) -> AteError {
        AteError::IO(err)
    }   
}

impl<E> From<InvokeError<E>>
for AteError
where E: std::fmt::Debug
{
    fn from(err: InvokeError<E>) -> AteError {
        AteError::InvokeError(err.to_string())
    }   
}

impl<E> From<ServiceError<E>>
for AteError
where E: std::fmt::Debug
{
    fn from(err: ServiceError<E>) -> AteError {
        AteError::ServiceError(err.to_string())
    }   
}

impl std::fmt::Display
for AteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AteError::BusError(err) => {
                write!(f, "{}", err)
            },
            AteError::ChainCreationError(err) => {
                write!(f, "{}", err)
            },
            AteError::CommitError(err) => {
                write!(f, "{}", err)
            },
            AteError::CommsError(err) => {
                write!(f, "{}", err)
            },
            AteError::CompactError(err) => {
                write!(f, "{}", err)
            },
            AteError::CryptoError(err) => {
                write!(f, "{}", err)
            },
            AteError::LintError(err) => {
                write!(f, "{}", err)
            },
            AteError::LoadError(err) => {
                write!(f, "{}", err)
            },
            AteError::LockError(err) => {
                write!(f, "{}", err)
            },
            AteError::ProcessError(err) => {
                write!(f, "{}", err)
            },
            AteError::SerializationError(err) => {
                write!(f, "{}", err)
            },
            AteError::SinkError(err) => {
                write!(f, "{}", err)
            },
            AteError::TimeError(err) => {
                write!(f, "{}", err)
            },
            AteError::TransformError(err) => {
                write!(f, "{}", err)
            },
            AteError::IO(err) => {
                write!(f, "{}", err)
            },
            AteError::InvokeError(err) => {
                write!(f, "{}", err)
            },
            AteError::ServiceError(err) => {
                write!(f, "{}", err)
            },
            AteError::NotImplemented => {
                write!(f, "Not implemented")
            },
        }
    }
}

impl std::error::Error
for AteError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub fn eat<T>(ret: Result<T, AteError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_load<T>(ret: Result<T, LoadError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_serialization<T>(ret: Result<T, SerializationError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_commit<T>(ret: Result<T, CommitError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_lock<T>(ret: Result<T, LockError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}