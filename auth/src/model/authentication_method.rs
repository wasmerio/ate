#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use serde::*;
use ate::crypto::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuthenticationMethod
{
    WithPrivateKey(PublicSignKey),
    WithPassword {
        salt: String,
        hash: AteHash,
    },
    WithAuthenticator {
        secret: String,
    },
    WithSmsAuthentication {
        salt: String,
        hash: AteHash,
    },
    WithEmailVerification {
        code: String,
    },
    WithSshKey {
        key_type: SshKeyType,
        secret: String,
    },
}