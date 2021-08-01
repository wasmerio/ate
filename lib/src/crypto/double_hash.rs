#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) struct DoubleHash {
    hash1: AteHash,
    hash2: AteHash,
}

impl DoubleHash {
    #[allow(dead_code)]
    pub fn from_hashes(hash1: &AteHash, hash2: &AteHash) -> DoubleHash {
        DoubleHash {
            hash1: hash1.clone(),
            hash2: hash2.clone(),
        }
    }

    pub fn hash(&self) -> AteHash {
        AteHash::from_bytes_twice(&self.hash1.val[..], &self.hash2.val[..])
    }
}