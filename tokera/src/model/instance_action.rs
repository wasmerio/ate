use serde::*;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum InstanceAction {
    Start,
    Stop,
    Restart,
    Kill,
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
            InstanceAction::Upgrade => write!(f, "upgrade"),
        }
    }
}