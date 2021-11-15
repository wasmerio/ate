use serde::{Deserialize, Serialize};

use crate::crypto::*;

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaConfidentiality {
    pub hash: ShortHash,
    #[serde(skip)]
    pub _cache: Option<ReadOption>,
}

impl std::fmt::Display for MetaConfidentiality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hash)
    }
}
