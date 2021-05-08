use serde::{Serialize, Deserialize};

use crate::trust::ChainEntropy;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MetaEntropy
{
    pub entropy: ChainEntropy,
}

impl std::fmt::Display
for MetaEntropy
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.entropy)
    }
}