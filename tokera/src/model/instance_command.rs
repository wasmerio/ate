use serde::*;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum InstanceCommand {
    Shell,
    WasmBus,
}

impl fmt::Display
for InstanceCommand
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceCommand::Shell => write!(f, "shell"),
            InstanceCommand::WasmBus => write!(f, "wasm-bus"),
        }
    }
}