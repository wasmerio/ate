#[allow(unused_imports)]
use log::{info, error, debug};
use std::error::Error;

extern crate rmp_serde as rmps;

use super::*;

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