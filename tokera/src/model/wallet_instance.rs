use serde::*;

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletInstance {
    /// Name of the instance attached to the identity
    pub name: String,
    /// The token is passed around by consumers to access the instance
    pub token: String,
    /// Name of the chain-of-trust used for this instance
    pub chain: String,
    /// The name of the web assembly package that is the code behind
    /// this running instance
    pub wapm: String,
}