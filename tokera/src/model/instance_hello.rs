use ate::chain::ChainKey;
use serde::*;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceHello {
    pub owner_identity: String,
    pub access_token: String,
    pub chain: ChainKey
}

impl fmt::Display
for InstanceHello
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "instance-hello(key={})", self.chain)
    }
}