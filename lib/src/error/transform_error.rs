#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::error::Error;
use crate::crypto::AteHash;

use super::*;

#[derive(Debug)]
pub enum TransformError {
    #[cfg(feature = "enable_openssl")]
    EncryptionError(openssl::error::ErrorStack),
    IO(std::io::Error),
    CryptoError(CryptoError),
    TrustError(TrustError),
    MissingReadKey(AteHash),
    UnspecifiedReadability,
}

#[cfg(feature = "enable_openssl")]
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

impl From<TrustError>
for TransformError
{
    fn from(err: TrustError) -> TransformError {
        TransformError::TrustError(err)
    }
}


impl std::fmt::Display
for TransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            #[cfg(feature = "enable_openssl")]
            TransformError::EncryptionError(err) => {
                write!(f, "Encryption error while transforming event data - {}", err)
            },
            TransformError::IO(err) => {
                write!(f, "IO error while transforming event data - {}", err)
            },
            TransformError::CryptoError(err) => {
                write!(f, "Cryptography error while transforming event data - {}", err)
            },
            TransformError::TrustError(err) => {
                write!(f, "Trust error while transforming event data - {}", err)
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