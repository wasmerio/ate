use serde::*;
use std::fmt;
use std::{fmt::Display, sync::Arc};
use wasmer_bus::macros::*;

#[wasmer_bus(format = "bincode")]
pub trait Pool {
    async fn spawn(
        &self,
        spawn: Spawn,
        stdout: impl Fn(Vec<u8>),
        stderr: impl Fn(Vec<u8>),
        exit: impl Fn(i32),
    ) -> Arc<dyn Process>;
}

#[wasmer_bus(format = "bincode")]
pub trait Process {
    async fn id(&self) -> u32;
    async fn stdin(&self, data: Vec<u8>) -> usize;
    async fn close_stdin(&self);
    async fn kill(&self);
    async fn flush(&self);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Spawn {
    pub path: String,
    pub args: Vec<String>,
    pub chroot: bool,
    pub working_dir: Option<String>,
    pub stdin_mode: StdioMode,
    pub stdout_mode: StdioMode,
    pub stderr_mode: StdioMode,
    pub pre_open: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StdioMode {
    Piped,
    Inherit,
    Null,
    Log,
}

impl Display for StdioMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StdioMode::Piped => write!(f, "piped"),
            StdioMode::Inherit => write!(f, "inherit"),
            StdioMode::Null => write!(f, "null"),
            StdioMode::Log => write!(f, "log"),
        }
    }
}
