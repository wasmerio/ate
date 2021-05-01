use serde::{Serialize, Deserialize};

use crate::crypto::*;

/// Determines if the event record will be restricted so that
/// only a specific set of users can read the data. If it is
/// limited to a specific set of users they must all possess
/// the encryption key in their session when accessing these
/// data records of which the hash of the encryption key must
/// match this record.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadOption
{
    Inherit,
    Everyone(Option<EncryptKey>),
    Specific(AteHash, DerivedEncryptKey)
}

impl ReadOption
{
    pub fn from_key(key: &EncryptKey) -> Result<ReadOption, std::io::Error> {
        Ok(ReadOption::Specific(key.hash(), DerivedEncryptKey::new(key)?))
    }
}

impl Default
for ReadOption
{
    fn default() -> ReadOption {
        ReadOption::Inherit
    }
}

impl std::fmt::Display
for ReadOption {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ReadOption::Everyone(key) => {
                if let Some(key) = key {
                    write!(f, "everyone({})", key.hash())
                } else {
                    write!(f, "everyone")
                }
            },
            ReadOption::Inherit => {
                write!(f, "inherit")
            },
            ReadOption::Specific(hash, _derived) => {
                write!(f, "specifc({})", hash)
            },
        }
    }
}