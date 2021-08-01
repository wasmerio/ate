#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use serde::*;
use ate::crypto::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authorization {
    pub read: EncryptKey,
    pub private_read: PrivateEncryptKey,
    pub write: PrivateSignKey,
}