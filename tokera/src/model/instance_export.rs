use ate::comms::NodeId;
use serde::*;

/// Exports are web assembly binaries that are exposed to the world
/// as consumable targets for anyone who possesses the access token
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceExport {
    /// Access token that allows access to this exported binary
    pub access_token: String,
    /// Name of the binary that can be invoked
    pub binary: String,
    /// Indicates if this export is fully distributed across the globe, if it is
    /// not then when it starts up then it pins itself to a particular location
    /// until it goes idle again.
    pub distributed: bool,
    /// Can be accessed via HTTP calls
    pub http: bool,
    /// Can be accessed via HTTPS calls
    pub https: bool,
    /// Can be accessed using the WASM-BUS
    pub bus: bool,
    /// Indicates where the service instance is currently pinned (when its stateful)
    pub pinned: Option<NodeId>,
}