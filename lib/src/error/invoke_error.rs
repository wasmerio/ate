#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use tokio::sync::mpsc as mpsc;

use super::*;

#[derive(Debug)]
pub enum InvokeError<E>
{
    IO(std::io::Error),
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

impl<E> From<std::io::Error>
for InvokeError<E>
{
    fn from(err: std::io::Error) -> InvokeError<E> {
        InvokeError::IO(err)
    }   
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