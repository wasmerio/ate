#[allow(unused_imports)]
use log::{info, error, debug};
use std::error::Error;
use crate::crypto::AteHash;

use super::*;

#[derive(Debug)]
pub enum SinkError {
    MissingPublicKey(AteHash),
    Trust(TrustError),
    InvalidSignature {
        hash: AteHash,
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