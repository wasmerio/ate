use error_chain::error_chain;
use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;
use serde_json::Error as JsonError;
use crate::prelude::PrimaryKey;

error_chain! {
    types {
        SerializationError, SerializationErrorKind, ResultExt, Result;
    }
    links {
    }
    foreign_links {
        IO(tokio::io::Error);
        EncodeError(RmpEncodeError);
        DecodeError(RmpDecodeError);
        JsonError(JsonError);
        BincodeError(bincode::Error);
    }
    errors {
        NoPrimarykey {
            description("data object does not have a primary key")
            display("data object does not have a primary key")
        }
        NoData {
            description("data object has no actual data")
            display("data object has no actual data")
        }
        InvalidSerializationFormat {
            description("data is stored in an unknown serialization format")
            display("data is stored in an unknown serialization format")
        }
        CollectionDetached {
            description("collection is detached from a parent")
            display("collection is detached from a parent")
        }
        SerdeError(err: String) {
            description("serde error during serialization"),
            display("serde error during serialization - {}", err),
        }
        WeakDio {
            description("the dio that created this object has gone out of scope")
            display("the dio that created this object has gone out of scope")
        }
        SaveParentFirst {
            description("you must save the parent object before attempting to push objects to this vector")
            display("you must save the parent object before attempting to push objects to this vector")
        }
        ObjectStillLocked(key: PrimaryKey) {
            description("data object with key is still being edited in the current scope"),
            display("data object with key ({}) is still being edited in the current scope", key.as_hex_string()),
        }
        AlreadyDeleted(key: PrimaryKey) {
            description("data object with key has already been deleted"),
            display("data object with key ({}) has already been deleted", key.as_hex_string()),
        }
    }
}