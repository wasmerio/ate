use serde::*;
use std::fmt;
use ate_crypto::ChainKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwitchHello {
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