use ate::chain::ChainKey;
use serde::*;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwitchHello {
    pub chain: ChainKey,
    pub access_token: String,    
}

impl fmt::Display
for SwitchHello
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "switch-hello(key={})", self.chain)
    }
}