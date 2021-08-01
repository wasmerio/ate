#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use std::error::Error;

use super::*;

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
    UrlError(url::ParseError),
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

impl From<serde_json::Error>
for AteError
{
    fn from(err: serde_json::Error) -> AteError {
        AteError::SerializationError(SerializationError::SerdeError(err.to_string()))
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

impl From<url::ParseError>
for AteError
{
    fn from(err: url::ParseError) -> AteError {
        AteError::UrlError(err)
    }   
}

impl From<tokio::sync::watch::error::RecvError>
for AteError
{
    fn from(err: tokio::sync::watch::error::RecvError) -> AteError {
        AteError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, err.to_string()))
    }   
}

impl<T> From<tokio::sync::watch::error::SendError<T>>
for AteError
where T: std::fmt::Debug
{
    fn from(err: tokio::sync::watch::error::SendError<T>) -> AteError {
        AteError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, err.to_string()))
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
            AteError::UrlError(err) => {
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