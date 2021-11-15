use ate::crypto::*;
use ate::prelude::*;
use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    pub purpose: AteRolePurpose,
    pub read: AteHash,
    pub private_read: PublicEncryptKey,
    pub write: PublicSignKey,
    pub access: MultiEncryptedSecureData<Authorization>,
}
