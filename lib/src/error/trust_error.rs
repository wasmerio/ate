use error_chain::error_chain;

use crate::header::PrimaryKey;

error_chain! {
    types {
        TrustError, TrustErrorKind, ResultExt, Result;
    }
    links {
        TimeError(super::TimeError, super::TimeErrorKind);
    }
    errors {
        NoAuthorizationWrite(key: PrimaryKey, write: crate::meta::WriteOption) {
            description("data object with key could not be written as the current session has no signature key for this authorization"),
            display("data object with key ({}) could not be written as the current session has no signature key for this authorization ({})", key.as_hex_string(), write),
        }
        NoAuthorizationRead(key: PrimaryKey, read: crate::meta::ReadOption) {
            description("data object with key could not be written as the current session has no encryption key for this authorization"),
            display("data object with key ({}) could not be written as the current session has no encryption key for this authorization ({})", key.as_hex_string(), read),
        }
        NoAuthorizationOrphan {
            description("data objects without a primary key has no write authorization")
            display("data objects without a primary key has no write authorization")
        }
        MissingParent(key: PrimaryKey) {
            description("data object references a parent object that does not exist"),
            display("data object references a parent object that does not exist ({})", key.as_hex_string()),
        }
        UnspecifiedWritability {
            description("the writability of this data object has not been specified")
            display("the writability of this data object has not been specified")
        }
    }
}