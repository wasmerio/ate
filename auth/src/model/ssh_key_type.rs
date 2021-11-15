use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SshKeyType {
    DSA,
    RSA,
    ED25519,
    ECDSA,
}
