use serde::*;

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletInstance {
    /// Name of the instance attached to the identity
    pub name: String,
    /// Name of the chain-of-trust used for this instance
    pub chain: String,
}