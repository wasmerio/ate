#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use std::error::Error;

use super::*;

#[derive(Debug)]
pub enum LockError
{
    SerializationError(SerializationError),
    LintError(LintError),
    CommitError(String),
    ReceiveError(String),
    WeakDio,
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
            LockError::WeakDio => {
                write!(f, "The DIO that created this object has gone out of scope")
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