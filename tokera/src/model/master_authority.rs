use ate::{prelude::*};
use serde::*;

/// Master authority is a row that holds the access rights
/// to one or more elements in a chain-of-trust. The keys
/// can be rotated periodically.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterAuthorityInner {
    /// Read key used access the service instance
    pub read: EncryptKey,
    /// Write key used to access the service instance
    pub write: PrivateSignKey,
}

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterAuthority {
    /// Inner area that is protected by a master key and is only accessible by the broker
    pub inner_broker: PublicEncryptedSecureData<MasterAuthorityInner>,
    /// Inner area that is protected by a master key and is only accessible by the owner
    pub inner_owner: PublicEncryptedSecureData<MasterAuthorityInner>,
}

impl std::fmt::Display
for MasterAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "master_authority(broker={},owner={})", self.inner_broker, self.inner_owner)
    }
}
