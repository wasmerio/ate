#[allow(unused_imports)]
use log::{info, error, debug};
use std::error::Error;
use crate::crypto::AteHash;
use crate::header::PrimaryKey;

extern crate rmp_serde as rmps;

use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;

use super::*;

#[derive(Debug)]
pub enum LoadError {
    NotFound(PrimaryKey),
    NoPrimaryKey,
    VersionMismatch,
    NotFoundByHash(AteHash),
    ObjectStillLocked(PrimaryKey),
    AlreadyDeleted(PrimaryKey),
    Tombstoned(PrimaryKey),
    SerializationError(SerializationError),
    TransformationError(TransformError),
    IO(tokio::io::Error),
    #[allow(dead_code)]
    CollectionDetached,
}

impl From<tokio::io::Error>
for LoadError
{
    fn from(err: tokio::io::Error) -> LoadError {
        LoadError::IO(err)
    }   
}

impl From<SerializationError>
for LoadError
{
    fn from(err: SerializationError) -> LoadError {
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

impl From<RmpEncodeError>
for LoadError {
    fn from(err: RmpEncodeError) -> LoadError {
        LoadError::SerializationError(SerializationError::EncodeError(err))
    }
}

impl From<RmpDecodeError>
for LoadError {
    fn from(err: RmpDecodeError) -> LoadError {
        LoadError::SerializationError(SerializationError::DecodeError(err))
    }
}

impl std::fmt::Display
for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoadError::NotFound(key) => {
                write!(f, "Data object with key ({}) could not be found", key.as_hex_string())
            },
            LoadError::NotFoundByHash(hash) => {
                write!(f, "Data object with hash ({}) could not be found", hash.to_string())
            },
            LoadError::VersionMismatch => {
                write!(f, "Entry has an invalid version for this log file")
            }
            LoadError::NoPrimaryKey => {
                write!(f, "Entry has no primary could and hence could not be loaded")
            },
            LoadError::ObjectStillLocked(key) => {
                write!(f, "Data object with key ({}) is still being edited in the current scope", key.as_hex_string())
            },
            LoadError::AlreadyDeleted(key) => {
                write!(f, "Data object with key ({}) has already been deleted", key.as_hex_string())
            },
            LoadError::Tombstoned(key) => {
                write!(f, "Data object with key ({}) has already been tombstoned", key.as_hex_string())
            },
            LoadError::SerializationError(err) => {
                write!(f, "Serialization error while attempting to load data object - {}", err)
            },
            LoadError::TransformationError(err) => {
                write!(f, "Transformation error while attempting to load data object - {}", err)
            },
            LoadError::IO(err) => {
                write!(f, "IO error while attempting to load data object - {}", err)
            },
            LoadError::CollectionDetached => {
                write!(f, "Collection is detached from its parent, it must be attached before it can be used")
            },
        }
    }
}

impl std::error::Error
for LoadError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}