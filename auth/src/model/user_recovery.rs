use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRecovery {
    pub email: String,
    pub google_auth: String,
    pub sudo_secret: String,
    pub qr_code: String,
    pub login_secret: EncryptKey,
}
