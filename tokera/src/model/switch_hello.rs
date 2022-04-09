use ate::chain::ChainKey;
use serde::*;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwitchHello {
    pub id: u128,
    pub chain: ChainKey,
    pub access_token: String,    
    pub version: u32,
}

impl fmt::Display
for SwitchHello
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "switch-hello(key={},version={})", self.chain, self.version)
    }
}