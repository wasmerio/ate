use serde::*;
use ate::prelude::ChainKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkToken {
    pub chain: ChainKey,
    pub access_token: String,
}