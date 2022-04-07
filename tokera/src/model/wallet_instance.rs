use serde::*;
use ate::chain::ChainKey;

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletInstance {
    /// Name of the instance attached to the identity
    pub name: String,
    /// ID of this instance within Tokera
    pub id: u128,
    /// Chain key for this service instance
    pub chain: ChainKey,
}

impl WalletInstance
{
    pub fn id_str(&self) -> String {
        hex::encode(&self.id.to_be_bytes())
    }
}