#[allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SshKeyType
{
    DSA,
    RSA,
    ED25519,
    ECDSA
}