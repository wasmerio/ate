#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::error::Error;

use tokio::sync::mpsc as mpsc;
use tokio::sync::broadcast as broadcast;

use super::*;

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
    LockError(CommsError),
    CommsError(CommsError),
    TimeError(TimeError),
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

impl From<TimeError>
for CommitError
{
    fn from(err: TimeError) -> CommitError {
        CommitError::TimeError(err)
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
            CommitError::LockError(err) => {
                write!(f, "Failed to lock the data due to an error in communication - {}", err.to_string())
            },
            CommitError::LintError(err) => {
                write!(f, "Failed to commit the data due to an error linting the data object events - {}", err.to_string())
            },
            CommitError::TimeError(err) => {
                write!(f, "Failed to commit the data due to an error in time keeping - {}", err.to_string())
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