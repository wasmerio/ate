use serde::*;
use ate::prelude::ChainKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkToken {
    pub chain: ChainKey,
    pub network_url: url::Url,
    pub access_token: String,
}