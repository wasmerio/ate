#[allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::*;
use ate::crypto::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authorization {
    pub read: EncryptKey,
    pub private_read: PrivateEncryptKey,
    pub write: PrivateSignKey,
}