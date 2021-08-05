#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use tokio::sync::mpsc as mpsc;

use super::*;

#[derive(Debug)]
pub enum InvokeError
{
    IO(std::io::Error),
    LoadError(LoadError),
    SerializationError(SerializationError),
    CommitError(CommitError),
    TransformError(TransformError),
    LockError(LockError),
    PipeError(String),
    ServiceError(String),
    Timeout,
    Aborted,
    NoData,
}

impl From<std::io::Error>
for InvokeError
{
    fn from(err: std::io::Error) -> InvokeError {
        InvokeError::IO(err)
    }   
}

impl From<SerializationError>
for InvokeError
{
    fn from(err: SerializationError) -> InvokeError {
        InvokeError::SerializationError(err)
    }   
}

impl From<LockError>
for InvokeError
{
    fn from(err: LockError) -> InvokeError {
        InvokeError::LockError(err)
    }   
}

impl<T> From<mpsc::error::SendError<T>>
for InvokeError
{
    fn from(err: mpsc::error::SendError<T>) -> InvokeError {
        InvokeError::PipeError(err.to_string())
    }   
}

impl From<LoadError>
for InvokeError
{
    fn from(err: LoadError) -> InvokeError {
        InvokeError::LoadError(err)
    }   
}

impl From<CommitError>
for InvokeError
{
    fn from(err: CommitError) -> InvokeError {
        InvokeError::CommitError(err)
    }   
}

impl From<TransformError>
for InvokeError
{
    fn from(err: TransformError) -> InvokeError {
        InvokeError::TransformError(err)
    }   
}

impl From<tokio::time::error::Elapsed>
for InvokeError
{
    fn from(_elapsed: tokio::time::error::Elapsed) -> InvokeError {
        InvokeError::Timeout
    }
}

impl std::fmt::Display
for InvokeError
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InvokeError::IO(err) => {
                write!(f, "Command failed - {}", err)
            }
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
            InvokeError::TransformError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::PipeError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::ServiceError(err) => {
                write!(f, "Command failed - {}", err)
            },
            InvokeError::Timeout => {
                write!(f, "Command failed - Timeout")
            },
            InvokeError::NoData => {
                write!(f, "Command failed - No Data")
            },
            InvokeError::Aborted => {
                write!(f, "Command failed - Aborted")
            },
        }
    }
}

impl std::error::Error
for InvokeError
{
}