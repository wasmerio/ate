#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use sha3::Digest;
use std::convert::TryInto;
use crate::utils::b16_serialize;
use crate::utils::b16_deserialize;
use crate::crypto::RandomGeneratorAccessor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HashRoutine
{
    Sha3,
    Blake3
}

/// Represents a hash of a piece of data that is cryptographically secure enough
/// that it can be used for integrity but small enough that it does not bloat
/// the redo log metadata.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AteHash {
    #[serde(serialize_with = "b16_serialize", deserialize_with = "b16_deserialize")]
    pub val: [u8; 16]
}

impl AteHash {
    pub fn generate() -> AteHash {
        RandomGeneratorAccessor::generate_hash()
    }
    pub fn from_hex_string(input: &str) -> Option<AteHash> {
        hex::decode(input.trim())
            .ok()
            .map(|a| {
                let bytes16: Option<[u8; 16]> = a.try_into().ok();
                bytes16
            })
            .flatten()
            .map(|a| {
                AteHash {
                    val: a
                }
            })
    }
    pub fn from_bytes(input: &[u8]) -> AteHash {
        Self::from_bytes_by_routine(input, crate::HASH_ROUTINE)
    }
    pub fn from_bytes_twice(input1: &[u8], input2: &[u8]) -> AteHash {
        Self::from_bytes_twice_by_routine(input1, input2, crate::HASH_ROUTINE)
    }
    fn from_bytes_by_routine(input: &[u8], routine: HashRoutine) -> AteHash {
        match routine {
            HashRoutine::Sha3 => AteHash::from_bytes_sha3(input, 1),
            HashRoutine::Blake3 => AteHash::from_bytes_blake3(input),
        }
    }
    fn from_bytes_twice_by_routine(input1: &[u8], input2: &[u8], routine: HashRoutine) -> AteHash {
        match routine {
            HashRoutine::Sha3 => AteHash::from_bytes_twice_sha3(input1, input2),
            HashRoutine::Blake3 => AteHash::from_bytes_twice_blake3(input1, input2),
        }
    }
    pub fn from_bytes_blake3(input: &[u8]) -> AteHash {
        let hash = blake3::hash(input);
        let bytes: [u8; 32] = hash.into();
        let mut bytes16: [u8; 16] = Default::default();
        bytes16.copy_from_slice(&bytes[0..16]);
        AteHash {
            val: bytes16
        }
    }
    fn from_bytes_twice_blake3(input1: &[u8], input2: &[u8]) -> AteHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(input1);
        hasher.update(input2);
        let hash = hasher.finalize();
        let bytes: [u8; 32] = hash.into();
        let mut bytes16: [u8; 16] = Default::default();
        bytes16.copy_from_slice(&bytes[0..16]);
        AteHash {
            val: bytes16
        }
    }
    pub fn from_bytes_sha3(input: &[u8], repeat: i32) -> AteHash {
        let mut hasher = sha3::Keccak384::default();
        for _ in 0..repeat {
            hasher.update(input);
        }
        let result = hasher.finalize();
        let result: Vec<u8> = result.into_iter()
            .take(16)
            .collect();
        let result: [u8; 16] = result
            .try_into()
            .expect("The hash should fit into 16 bytes!");

        AteHash {
            val: result,
        }
    }
    fn from_bytes_twice_sha3(input1: &[u8], input2: &[u8]) -> AteHash {
        let mut hasher = sha3::Keccak384::default();
        hasher.update(input1);
        hasher.update(input2);
        let result = hasher.finalize();
        let result: Vec<u8> = result.into_iter()
            .take(16)
            .collect();
        let result: [u8; 16] = result
            .try_into()
            .expect("The hash should fit into 16 bytes!");

        AteHash {
            val: result,
        }
    }
    
    pub fn to_u64(&self) -> u64 {
        let mut val = [0u8; 8];
        val.copy_from_slice(&self.val[..8]);
        u64::from_be_bytes(val)
    }

    pub fn to_hex_string(&self) -> String {
        hex::encode(self.val)
    }

    pub fn to_4hex(&self) -> String {
        let ret = hex::encode(self.val);
        format!("{}", &ret[..4])
    }

    pub fn to_8hex(&self) -> String {
        let ret = hex::encode(self.val);
        format!("{}", &ret[..8])
    }

    pub fn to_string(&self) -> String {
        self.to_hex_string()
    }

    pub fn to_base64(&self) -> String {
        base64::encode(&self.val[..])
    }

    pub fn to_bytes(&self) -> &[u8; 16] {
        &self.val
    }
}

impl From<String>
for AteHash
{
    fn from(val: String) -> AteHash {
        AteHash::from_bytes(val.as_bytes())
    }
}

impl From<&'static str>
for AteHash
{
    fn from(val: &'static str) -> AteHash {
        AteHash::from(val.to_string())
    }
}

impl From<u64>
for AteHash
{
    fn from(val: u64) -> AteHash {
        AteHash::from_bytes(&val.to_be_bytes())
    }
}

impl std::fmt::Display for AteHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}