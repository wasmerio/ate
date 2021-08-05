#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

#[derive(Debug)]
pub enum CryptoError {
    NoIvPresent,    
}

impl From<CryptoError>
for std::io::Error {
    fn from(error: CryptoError) -> Self {
        match error {
            CryptoError::NoIvPresent => std::io::Error::new(std::io::ErrorKind::Other, "The metadata does not have IV component present")
        }
    }
}

impl std::fmt::Display
for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CryptoError::NoIvPresent => {
                write!(f, "The event has no initialization vector")
            },
        }
    }
}

impl std::error::Error
for CryptoError
{
}