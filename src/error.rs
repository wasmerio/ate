#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum CryptoError {
    NoIvPresent,    
}

impl From<CryptoError>
for std::io::Error {
    fn from(error: CryptoError) -> Self {
        match error {
            CryptoError::NoIvPresent => std::io::Error::new(std::io::ErrorKind::Other, "The metadata does not have IV component present")
        }
    }
}