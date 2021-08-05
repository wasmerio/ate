use error_chain::error_chain;

use crate::crypto::AteHash;

error_chain! {
    types {
        SinkError, SinkErrorKind, ResultExt, Result;
    }
    links {
        TrustError(super::TrustError, super::TrustErrorKind);
    }
    errors {
        MissingPublicKey(hash: AteHash) {
            description("the public key for signature could not be found in the chain-of-trust"),
            display("the public key ({}) for signature could not be found in the chain-of-trust", hash.to_string()),
        }
        InvalidSignature(hash: AteHash, err: Option<pqcrypto_traits::Error>) {
            description("failed verification of hash while using public key"),
            display("failed verification of hash while using public key ({}) - {}", hash.to_string(), err.map(|a| a.to_string()).unwrap_or_else(|| "unknown reason".to_string()))
        }
    }
}