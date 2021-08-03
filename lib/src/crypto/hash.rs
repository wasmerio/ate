#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use sha3::Digest;
use std::convert::TryInto;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HashRoutine
{
    Sha3,
}

/// Represents a hash of a piece of data that is cryptographically secure enough
/// that it can be used for integrity but small enough that it does not bloat
/// the redo log metadata.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AteHash {
    pub val: [u8; 16]
}

impl AteHash {
    pub fn from_bytes(input: &[u8]) -> AteHash {
        Self::from_bytes_by_routine(input, crate::HASH_ROUTINE)
    }
    pub fn from_bytes_twice(input1: &[u8], input2: &[u8]) -> AteHash {
        Self::from_bytes_twice_by_routine(input1, input2, crate::HASH_ROUTINE)
    }
    fn from_bytes_by_routine(input: &[u8], routine: HashRoutine) -> AteHash {
        match routine {
            HashRoutine::Sha3 => AteHash::from_bytes_sha3(input, 1),
        }
    }
    fn from_bytes_twice_by_routine(input1: &[u8], input2: &[u8], routine: HashRoutine) -> AteHash {
        match routine {
            HashRoutine::Sha3 => AteHash::from_bytes_twice_sha3(input1, input2),
        }
    }
    pub fn from_bytes_sha3(input: &[u8], repeat: i32) -> AteHash {
        let mut hasher = sha3::Keccak384::new();
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
        let mut hasher = sha3::Keccak384::new();
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