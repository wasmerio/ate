#[allow(unused_imports)]
use log::{info, error, debug};
use serde::*;

use super::ChainEntropy;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainHeader
{
    // Last known entropy when the redo-log was created
    pub entropy: ChainEntropy,
}