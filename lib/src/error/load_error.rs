use error_chain::error_chain;
use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;

use crate::header::PrimaryKey;
use crate::crypto::AteHash;

error_chain! {
    types {
        LoadError, LoadErrorKind, ResultExt, Result;
    }
    links {
        SerializationError(super::SerializationError, super::SerializationErrorKind);
        TransformationError(super::TransformError, super::TransformErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        NotFound(key: PrimaryKey) {
            description("data object with key could not be found"),
            display("data object with key ({}) could not be found", key.as_hex_string()),
        }
        NoPrimaryKey {
            description("entry has no primary could and hence could not be loaded")
            display("entry has no primary could and hence could not be loaded")
        }
        VersionMismatch {
            description("entry has an invalid version for this log file")
            display("entry has an invalid version for this log file")
        }
        NotFoundByHash(hash: AteHash) {
            description("data object with hash could not be found"),
            display("data object with hash ({}) could not be found", hash.to_string()),
        }
        ObjectStillLocked(key: PrimaryKey) {
            description("data object with key is still being edited in the current scope"),
            display("data object with key ({}) is still being edited in the current scope", key.as_hex_string()),
        }
        AlreadyDeleted(key: PrimaryKey) {
            description("data object with key has already been deleted"),
            display("data object with key ({}) has already been deleted", key.as_hex_string()),
        }
        Tombstoned(key: PrimaryKey) {
            description("data object with key has already been tombstoned"),
            display("data object with key ({}) has already been tombstoned", key.as_hex_string()),
        }
        ChainCreationError(err: String) {
            description("chain creation error while attempting to load data object"),
            display("chain creation error while attempting to load data object - {}", err),
        }
        NoRepository {
            description("chain has no repository thus could not load foreign object")
            display("chain has no repository thus could not load foreign object")
        }
        CollectionDetached {
            description("collection is detached from its parent, it must be attached before it can be used")
            display("collection is detached from its parent, it must be attached before it can be used")
        }
        WeakDio {
            description("the dio that created this object has gone out of scope")
            display("the dio that created this object has gone out of scope")
        }
    }
}

impl From<RmpEncodeError>
for LoadError {
    fn from(err: RmpEncodeError) -> LoadError {
        LoadErrorKind::SerializationError(super::SerializationErrorKind::EncodeError(err).into()).into()
    }
}

impl From<RmpDecodeError>
for LoadError {
    fn from(err: RmpDecodeError) -> LoadError {
        LoadErrorKind::SerializationError(super::SerializationErrorKind::DecodeError(err).into()).into()
    }
}

impl From<super::ChainCreationError>
for LoadError
{
    fn from(err: super::ChainCreationError) -> LoadError {
        LoadErrorKind::ChainCreationError(err.to_string()).into()
    }   
}

impl From<super::ChainCreationErrorKind>
for LoadError
{
    fn from(err: super::ChainCreationErrorKind) -> LoadError {
        LoadErrorKind::ChainCreationError(err.to_string()).into()
    }   
}