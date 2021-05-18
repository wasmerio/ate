#[allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::*;
use ate::crypto::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Advert {
    pub email: String,
    pub uid: u32,
    pub nominal_encrypt: PublicEncryptKey,
    pub nominal_auth: PublicSignKey,
    pub sudo_encrypt: PublicEncryptKey,
    pub sudo_auth: PublicSignKey,
}