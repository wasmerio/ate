#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use crate::crypto::AteHash;

use super::*;

#[derive(Debug)]
pub enum LintError {
    IO(std::io::Error),
    MissingWriteKey(AteHash),
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
}