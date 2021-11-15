use ate::prelude::*;
use serde::*;

use super::*;

/// Represents ownership of a particular commodity. This is normally
/// used in a person of groups wallet so that they can access a
/// particular asset or redeem it
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ownership {
    pub kind: CommodityKind,
    pub chain: ChainKey,
    pub what: PrimaryKey,
    pub token: EncryptKey,
}
