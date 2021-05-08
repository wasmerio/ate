#[allow(unused_imports)]
use log::{info, error, debug};
use serde::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainEntropy
{
    pub entropy: u64,
}

impl Default
for ChainEntropy
{
    fn default() -> ChainEntropy {
        ChainEntropy {
            entropy: 0u64,
        }
    }
}

impl From<u64>
for ChainEntropy
{
    fn from(val: u64) -> ChainEntropy {
        ChainEntropy {
            entropy: val
        }
    }
}

impl std::fmt::Display
for ChainEntropy
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.entropy)
    }
}