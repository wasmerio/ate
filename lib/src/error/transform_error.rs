use error_chain::error_chain;

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
        MissingData {
            description("missing data for this record")
            display("missing data for this record")
        }
        MissingReadKey(hash: String) {
            description("missing the read key needed to encrypt/decrypt this data object"),
            display("missing the read key ({}) needed to encrypt/decrypt this data object", hash)
        }
        UnspecifiedReadability {
            description("the readability for this data object has not been specified")
            display("the readability for this data object has not been specified")
        }
    }
}

#[cfg(feature = "enable_openssl")]
impl From<openssl::error::ErrorStack> for Error {
    fn from(err: openssl::error::ErrorStack) -> Error {
        ErrorKind::EncryptionError(err).into()
    }
}
