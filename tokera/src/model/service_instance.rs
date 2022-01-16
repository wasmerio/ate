use serde::*;
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus
{
    Running,
    Stopped,
}

/// Running instance of a particular web assembly application
/// within the hosting environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInstance {
    /// Token associated with this service instance
    pub token: String,
    /// The current status of this service instance
    pub status: InstanceStatus,
    /// The name of the web assembly package that is the code behind
    /// this running instance
    pub wapm: String,
    /// Identity of the owner of this particular service (the owner of
    /// the service is able to incur charges on your wallet)
    pub owner_identity: String,
}

impl fmt::Display
for InstanceStatus
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStatus::Running => write!(f, "running"),
            InstanceStatus::Stopped => write!(f, "stopped"),
        }
    }
}