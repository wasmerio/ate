use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::*;
use ate::crypto::EncryptKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sudo {
    pub email: String,
    pub uid: u32,
    pub google_auth: String,
    pub secret: String,
    pub qr_code: String,
    pub failed_attempts: u32,
    pub contract_read_key: EncryptKey,
    pub access: Vec<Authorization>,
    pub groups: Vec<String>,
}
