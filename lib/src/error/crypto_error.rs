use error_chain::error_chain;

error_chain! {
    types {
        CryptoError, CryptoErrorKind, ResultExt, Result;
    }
    errors {
        NoIvPresent {
            display("no initialization vector")
        }
    }
}

impl From<CryptoError>
for std::io::Error {
    fn from(error: CryptoError) -> Self {
        match error {
            CryptoError(CryptoErrorKind::NoIvPresent, _) => std::io::Error::new(std::io::ErrorKind::Other, "The metadata does not have IV component present"),
            _ => std::io::Error::new(std::io::ErrorKind::Other, "An unknown error occured while performing ate crypto")
        }
    }
}