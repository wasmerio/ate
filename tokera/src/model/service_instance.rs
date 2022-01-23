use ate::{prelude::DaoVec};
use serde::*;

use super::{InstanceExport};

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInstance {
    /// Name of the instance attached to the identity
    pub name: String,
    /// Name of the chain-of-trust used for this instance
    pub chain: String,
    /// List of all the binaries that are exposed by this instance
    /// and hence can be invoked by clients
    pub exports: DaoVec<InstanceExport>,
}