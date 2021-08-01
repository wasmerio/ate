#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SshKeyType
{
    DSA,
    RSA,
    ED25519,
    ECDSA
}