use ate::crypto::*;
use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuthenticationMethod {
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
