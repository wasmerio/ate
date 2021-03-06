use super::crypto::Hash;

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

#[derive(Debug)]
pub enum TransformError {
    EncryptionError(openssl::error::ErrorStack),
    IO(std::io::Error),
    CryptoError(CryptoError),
}

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

impl std::fmt::Display
for TransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TransformError::EncryptionError(err) => {
                write!(f, "Encryption error while transforming event data - {}", err)
            },
            TransformError::IO(err) => {
                write!(f, "IO error while transforming event data - {}", err)
            },
            TransformError::CryptoError(err) => {
                write!(f, "Cryptography error while transforming event data - {}", err)
            },
        }
    }
}

#[derive(Debug)]
pub enum CompactError {
    SinkError(SinkError),
    IO(tokio::io::Error),
    SerializationError(EventSerializationError),
}

impl From<tokio::io::Error>
for CompactError {
    fn from(err: tokio::io::Error) -> CompactError {
        CompactError::IO(err)
    }
}

impl From<SinkError>
for CompactError {
    fn from(err: SinkError) -> CompactError {
        CompactError::SinkError(err)
    }
}

impl From<EventSerializationError>
for CompactError {
    fn from(err: EventSerializationError) -> CompactError {
        CompactError::SerializationError(err)
    }
}

#[derive(Debug)]
pub enum SinkError {
    MissingPublicKey(Hash),
    InvalidSignature {
        hash: Hash,
        err: Option<pqcrypto_traits::Error>,
    }
}

impl std::fmt::Display
for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SinkError::MissingPublicKey(hash) => {
                write!(f, "The public key ({}) for signature could not be found in the chain-of-trust", hash.to_string())
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

#[derive(Debug)]
pub enum EventSerializationError
{
    NoPrimarykey,
    NoData,
    BincodeError(bincode::Error),
}

impl From<bincode::Error>
for EventSerializationError {
    fn from(err: bincode::Error) -> EventSerializationError {
        EventSerializationError::BincodeError(err)
    }
}

#[derive(Debug)]
pub enum LoadError {
    NotFound,
    Locked,
    AlreadyDeleted,
    InternalError(String),
    Tombstoned,
    SerializationError(EventSerializationError),
    TransformationError(TransformError),
    IO(tokio::io::Error),
}

impl From<tokio::io::Error>
for LoadError
{
    fn from(err: tokio::io::Error) -> LoadError {
        LoadError::IO(err)
    }   
}

impl From<EventSerializationError>
for LoadError
{
    fn from(err: EventSerializationError) -> LoadError {
        LoadError::SerializationError(err)
    }   
}

impl From<TransformError>
for LoadError
{
    fn from(err: TransformError) -> LoadError {
        LoadError::TransformationError(err)
    }   
}

#[derive(Debug)]
pub enum FeedError {
    SinkError(SinkError),
    IO(tokio::io::Error),
}

impl From<SinkError>
for FeedError
{
    fn from(err: SinkError) -> FeedError {
        FeedError::SinkError(err)
    }   
}

impl From<tokio::io::Error>
for FeedError
{
    fn from(err: tokio::io::Error) -> FeedError {
        FeedError::IO(err)
    }   
}

impl std::fmt::Display
for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FeedError::SinkError(err) => {
                write!(f, "Event sink error while processing a stream of events- {}", err)
            },
            FeedError::IO(err) => {
                write!(f, "IO sink error while processing a stream of events- {}", err)
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct ProcessError
{
    pub sink_errors: Vec<SinkError>,
}

impl ProcessError {
    pub fn has_errors(&self) -> bool {
        if self.sink_errors.is_empty() == false { return true; }
        false
    }

    pub fn as_result(self) -> Result<(), ProcessError> {
        match self.has_errors() {
            true => Err(self),
            false => Ok(())
        }
    }
}

#[derive(Debug)]
pub enum ChainCreationError {
    ProcessError(ProcessError),
    IO(tokio::io::Error),
    SerializationError(EventSerializationError),
}

impl From<ProcessError>
for ChainCreationError
{
    fn from(err: ProcessError) -> ChainCreationError {
        ChainCreationError::ProcessError(err)
    }   
}

impl From<EventSerializationError>
for ChainCreationError
{
    fn from(err: EventSerializationError) -> ChainCreationError {
        ChainCreationError::SerializationError(err)
    }   
}

impl From<tokio::io::Error>
for ChainCreationError
{
    fn from(err: tokio::io::Error) -> ChainCreationError {
        ChainCreationError::IO(err)
    }   
}