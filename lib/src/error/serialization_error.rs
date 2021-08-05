use error_chain::error_chain;
use rmp_serde::encode::Error as RmpEncodeError;
use rmp_serde::decode::Error as RmpDecodeError;
use serde_json::Error as JsonError;

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
            display("data object does not have a primary key")
        }
        NoData {
            display("data object has no actual data")
        }
        InvalidSerializationFormat {
            display("data is stored in an unknown serialization format")
        }
        CollectionDetached {
            display("collection is detached from a parent")
        }
        SerdeError(err: String) {
            description("serde error during serialization"),
            display("serde error during serialization - {}", err),
        }
        WeakDio {
            display("the dio that created this object has gone out of scope")
        }
        SaveParentFirst {
            display("you must save the parent object before attempting to push objects to this vector")
        }
    }
}