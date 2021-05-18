#[allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::*;
use ate::crypto::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmsVerification {
    pub salt: String,
    pub hash: AteHash,
}