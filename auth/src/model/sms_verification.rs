#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::crypto::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmsVerification {
    pub salt: String,
    pub hash: AteHash,
}