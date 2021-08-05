use error_chain::error_chain;
use crate::crypto::AteHash;

error_chain! {
    types {
        TransformError, TransformErrorKind, ResultExt, Result;
    }
    links {
        CryptoError(super::CryptoError, super::CryptoErrorKind);
        TrustError(super::TrustError, super::TrustErrorKind);
    }
    foreign_links {
        IO(std::io::Error);
    }
    errors {
        #[cfg(feature = "enable_openssl")]
        EncryptionError(stack: openssl::error::ErrorStack) {
            description("encryption error while transforming event data"),
            display("encryption error while transforming event data - {}", err),
        }
        MissingReadKey(hash: AteHash) {
            description("missing the read key needed to encrypt/decrypt this data object"),
            display("missing the read key ({}) needed to encrypt/decrypt this data object", hash.to_string())
        }
        UnspecifiedReadability {
            display("the readability for this data object has not been specified")
        }
    }
}

#[cfg(feature = "enable_openssl")]
impl From<openssl::error::ErrorStack>
for Error
{
    fn from(err: openssl::error::ErrorStack) -> Error {
        ErrorKind::EncryptionError(err).into()
    }
}