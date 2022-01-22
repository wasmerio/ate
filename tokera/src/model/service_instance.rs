use serde::*;
use std::fmt;

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
    /// Next action to perform on this instance
    pub action: Option<InstanceAction>,
    /// The current status of this service instance
    pub status: InstanceStatus,
    /// The name of the web assembly package that is the code behind
    /// this running instance
    pub wapm: String,
    /// Indicates if this service instance is stateful or not
    pub stateful: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus
{
    Idle,
    Running,
    Polling,
    Compiling,
    Stopped,
    Upgrading,
}

impl fmt::Display
for InstanceStatus
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStatus::Idle => write!(f, "idle"),
            InstanceStatus::Running => write!(f, "running"),
            InstanceStatus::Polling => write!(f, "polling"),
            InstanceStatus::Compiling => write!(f, "compiling"),
            InstanceStatus::Upgrading => write!(f, "upgrading"),
            InstanceStatus::Stopped => write!(f, "stopped"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum InstanceAction {
    Start,
    Stop,
    Restart,
    Kill,
    Clone,
    Backup {
        chain: String,
        path: String,
    },
    Restore {
        chain: String,
        path: String
    },
    Upgrade,
}

impl fmt::Display
for InstanceAction
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceAction::Start => write!(f, "start"),
            InstanceAction::Stop => write!(f, "stop"),
            InstanceAction::Restart => write!(f, "restart"),
            InstanceAction::Kill => write!(f, "kill"),
            InstanceAction::Clone => write!(f, "clone"),
            InstanceAction::Backup { .. } => write!(f, "backup"),
            InstanceAction::Restore { .. } => write!(f, "restore"),
            InstanceAction::Upgrade => write!(f, "upgrade"),
        }
    }
}