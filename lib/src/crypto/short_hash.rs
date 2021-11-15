use serde::{Deserialize, Serialize};
use sha3::Digest;
use std::convert::TryInto;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::crypto::HashRoutine;

/// Represents a hash of a piece of data that is cryptographically secure enough
/// that it can be used for integrity but small enough that it does not bloat
/// the redo log metadata.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ShortHash {
    pub val: u32,
}

impl ShortHash {
    pub fn from_bytes(input: &[u8]) -> ShortHash {
        Self::from_bytes_by_routine(input, crate::HASH_ROUTINE)
    }

    pub fn from_bytes_twice(input1: &[u8], input2: &[u8]) -> ShortHash {
        Self::from_bytes_twice_by_routine(input1, input2, crate::HASH_ROUTINE)
    }

    fn from_bytes_by_routine(input: &[u8], routine: HashRoutine) -> ShortHash {
        match routine {
            HashRoutine::Sha3 => ShortHash::from_bytes_sha3(input, 1),
            HashRoutine::Blake3 => ShortHash::from_bytes_blake3(input),
        }
    }

    fn from_bytes_twice_by_routine(
        input1: &[u8],
        input2: &[u8],
        routine: HashRoutine,
    ) -> ShortHash {
        match routine {
            HashRoutine::Sha3 => ShortHash::from_bytes_twice_sha3(input1, input2),
            HashRoutine::Blake3 => ShortHash::from_bytes_twice_blake3(input1, input2),
        }
    }

    pub fn from_bytes_blake3(input: &[u8]) -> ShortHash {
        let hash = blake3::hash(input);
        let bytes: [u8; 32] = hash.into();
        let mut bytes4: [u8; 4] = Default::default();
        bytes4.copy_from_slice(&bytes[0..4]);
        ShortHash {
            val: u32::from_be_bytes(bytes4),
        }
    }

    fn from_bytes_twice_blake3(input1: &[u8], input2: &[u8]) -> ShortHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(input1);
        hasher.update(input2);
        let hash = hasher.finalize();
        let bytes: [u8; 32] = hash.into();
        let mut bytes4: [u8; 4] = Default::default();
        bytes4.copy_from_slice(&bytes[0..4]);
        ShortHash {
            val: u32::from_be_bytes(bytes4),
        }
    }

    pub fn from_bytes_sha3(input: &[u8], repeat: i32) -> ShortHash {
        let mut hasher = sha3::Keccak384::new();
        for _ in 0..repeat {
            hasher.update(input);
        }
        let result = hasher.finalize();
        let result: Vec<u8> = result.into_iter().take(4).collect();
        let result: [u8; 4] = result
            .try_into()
            .expect("The hash should fit into 4 bytes!");
        let result = u32::from_be_bytes(result);

        ShortHash { val: result }
    }

    fn from_bytes_twice_sha3(input1: &[u8], input2: &[u8]) -> ShortHash {
        let mut hasher = sha3::Keccak384::new();
        hasher.update(input1);
        hasher.update(input2);
        let result = hasher.finalize();
        let result = result.iter().take(4).map(|b| *b).collect::<Vec<_>>();
        let result: [u8; 4] = result
            .try_into()
            .expect("The hash should fit into 4 bytes!");
        let result = u32::from_be_bytes(result);

        ShortHash { val: result }
    }

    pub fn to_hex_string(&self) -> String {
        hex::encode(self.val.to_be_bytes())
    }

    pub fn to_string(&self) -> String {
        self.to_hex_string()
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.val.to_be_bytes()
    }
}

impl From<String> for ShortHash {
    fn from(val: String) -> ShortHash {
        ShortHash::from_bytes(val.as_bytes())
    }
}

impl From<&'static str> for ShortHash {
    fn from(val: &'static str) -> ShortHash {
        ShortHash::from(val.to_string())
    }
}

impl From<u64> for ShortHash {
    fn from(val: u64) -> ShortHash {
        ShortHash::from_bytes(&val.to_be_bytes())
    }
}

impl std::fmt::Display for ShortHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
