use ate::{prelude::DaoVec};
use serde::*;

use super::{InstanceExport, InstanceSubnet, MeshNode};

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInstance {
    /// Unique ID of this instance
    pub id: u128,
    /// Chain key for this service instance
    pub chain: String,
    /// Subnet associated with this instance
    pub subnet: InstanceSubnet,
    /// Admin token associated with an instance
    pub admin_token: String,
    /// List of all the binaries that are exposed by this instance
    /// and hence can be invoked by clients
    pub exports: DaoVec<InstanceExport>,
    /// List of active nodes currently partipating in the mesh
    pub mesh_nodes: DaoVec<MeshNode>,
}

impl ServiceInstance
{
    pub fn id_str(&self) -> String {
        hex::encode(&self.id.to_be_bytes())
    }
}