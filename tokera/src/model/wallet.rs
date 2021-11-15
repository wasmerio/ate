use ate::prelude::*;
use serde::*;

use super::*;

/// Wallets are attached to users (persons) or to groups
/// which allows ownership of commodities. Proof of ownership
/// can be used to access or consume commodities which can be
/// achieved by proving ownership of the wallet itself. Attaching
/// the wallet is done by making it a child of user/group.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Wallet {
    /// Name that can be associated with the wallet for organization purposes (default=default)
    pub name: String,
    /// The country you are resident in for tax purposes
    pub gst_country: Country,
    /// All the coins that need to be processed again (e.g. deposits)
    pub inbox: DaoVec<Ownership>,
    /// Represents all the bags of coins
    pub bags: DaoMap<Denomination, BagOfCoins>,
    /// Represents everything that has happened in this wallet
    pub history: DaoVec<HistoricMonth>,
    /// This secure broker key is used by Tokera to access this wallet but can only
    /// be acquired by the owner of the wallet sharing it (given that Tokera does not store it)
    pub broker_key: EncryptKey,
    /// The broker unlock key needs to be supplied in order to access the broker key
    /// (this one is given to the broker key - which makes creates the trust trinity)
    pub broker_unlock_key: EncryptKey,
}
