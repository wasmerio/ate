use serde::*;
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus
{
    Idle,
    Running,
    Compiling,
    Stopping,
    Stopped,
    Upgrading,
}

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInstance {
    /// Name of the instance attached to the identity
    pub name: String,
    /// The token is passed around by consumers to access the instance
    pub token: String,
    /// Name of the chain-of-trust used for this instance
    pub chain: String,
    /// The current status of this service instance
    pub status: InstanceStatus,
    /// The name of the web assembly package that is the code behind
    /// this running instance
    pub wapm: String,
    /// Indicates if this service instance is stateful or not
    pub stateful: bool,
}

impl fmt::Display
for InstanceStatus
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStatus::Idle => write!(f, "idle"),
            InstanceStatus::Running => write!(f, "running"),
            InstanceStatus::Compiling => write!(f, "compiling"),
            InstanceStatus::Upgrading => write!(f, "upgrading"),
            InstanceStatus::Stopping => write!(f, "stopping"),
            InstanceStatus::Stopped => write!(f, "stopped"),
        }
    }
}