#[allow(unused_imports)]
use log::{info, error, debug};
use serde::{Serialize, Deserialize};
use rand::{RngCore};

use super::*;

/// Represents an initiailization vector used for both hash prefixing
/// to create entropy and help prevent rainbow table attacks. These
/// vectors are also used as the exchange medium during a key exchange
/// so that two parties can established a shared secret key
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct InitializationVector
{
    pub bytes: Vec<u8>,
}

impl InitializationVector {
    pub fn generate() -> InitializationVector {
        let mut rng = RandomGeneratorAccessor::default();
        let mut iv = InitializationVector {
            bytes: vec![0 as u8; 16]
        };
        rng.fill_bytes(&mut iv.bytes);
        iv
    }
}

impl From<Vec<u8>>
for InitializationVector
{
    fn from(bytes: Vec<u8>) -> InitializationVector {
        InitializationVector {
            bytes,
        }
    }
}

impl From<&[u8]>
for InitializationVector
{
    fn from(bytes: &[u8]) -> InitializationVector {
        InitializationVector {
            bytes: bytes.to_vec(),
        }
    }
}

impl From<&[u8; 16]>
for InitializationVector
{
    fn from(bytes: &[u8; 16]) -> InitializationVector {
        InitializationVector {
            bytes: bytes.to_vec(),
        }
    }
}

impl std::fmt::Display
for InitializationVector
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.bytes[..]))
    }
}