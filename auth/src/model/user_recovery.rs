#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;

use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRecovery {
    pub email: String,
    pub google_auth: String,
    pub sudo_secret: String,
    pub qr_code: String,
    pub login_secret: EncryptKey,
}