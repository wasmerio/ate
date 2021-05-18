#[allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::*;
use ate::crypto::*;
use ate::prelude::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    pub purpose: AteRolePurpose,
    pub read: AteHash,
    pub private_read: PublicEncryptKey,
    pub write: PublicSignKey,
    pub access: MultiEncryptedSecureData<Authorization>,
}