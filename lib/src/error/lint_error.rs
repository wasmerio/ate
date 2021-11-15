use error_chain::error_chain;

use crate::crypto::AteHash;

error_chain! {
    types {
        LintError, LintErrorKind, ResultExt, Result;
    }
    links {
        TrustError(super::TrustError, super::TrustErrorKind);
        TimeError(super::TimeError, super::TimeErrorKind);
        SerializationError(super::SerializationError, super::SerializationErrorKind);
    }
    foreign_links {
        IO(std::io::Error);
    }
    errors {
        MissingWriteKey(hash: AteHash) {
            description("could not find the write public key in the session"),
            display("could not find the write public key ({}) in the session", hash.to_string()),
        }
    }
}
