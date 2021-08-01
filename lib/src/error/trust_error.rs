#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use std::error::Error;
use crate::header::PrimaryKey;

use super::*;

#[derive(Debug)]
pub enum TrustError
{
    NoAuthorizationWrite(PrimaryKey, crate::meta::WriteOption),
    NoAuthorizationRead(PrimaryKey, crate::meta::ReadOption),
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
            TrustError::NoAuthorizationWrite(key, write) => {
                write!(f, "Data object with key ({}) could not be written as the current session has no signature key for this authorization ({})", key.as_hex_string(), write)
            },
            TrustError::NoAuthorizationRead(key, read) => {
                write!(f, "Data object with key ({}) could not be written as the current session has no encryption key for this authorization ({})", key.as_hex_string(), read)
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