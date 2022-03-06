use ate::chain::ChainKey;
use serde::*;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceHello {
    pub chain: ChainKey,
    pub access_token: String,    
}

impl fmt::Display
for InstanceHello
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "instance-hello(key={})", self.chain)
    }
}