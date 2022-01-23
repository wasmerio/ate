use serde::*;
use std::fmt;

use super::InstanceAction;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum InstanceCommand {
    Action(InstanceAction),
    Shell,
    WasmBus,
}

impl fmt::Display
for InstanceCommand
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceCommand::Action(action) => write!(f, "action({})", action),
            InstanceCommand::Shell => write!(f, "shell"),
            InstanceCommand::WasmBus => write!(f, "wasm-bus"),
        }
    }
}