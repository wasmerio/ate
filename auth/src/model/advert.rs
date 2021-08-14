#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::crypto::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AdvertId
{
    UID(u32),
    GID(u32)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Advert {
    pub identity: String,
    pub id: AdvertId,
    pub nominal_encrypt: PublicEncryptKey,
    pub nominal_auth: PublicSignKey,
    pub sudo_encrypt: PublicEncryptKey,
    pub sudo_auth: PublicSignKey,
    pub broker_encrypt: PublicEncryptKey,
    pub broker_auth: PublicSignKey,
}