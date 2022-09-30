use ate::crypto::*;
use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authorization {
    pub read: EncryptKey,
    pub private_read: PrivateEncryptKey,
    pub write: PrivateSignKey,
}
