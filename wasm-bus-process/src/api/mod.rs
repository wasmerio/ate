use serde::*;
use wasm_bus::macros::*;

#[wasm_bus(format = "bincode")]
pub trait Pool {
    fn spawn(
        spawn: Spawn,
        stdout: dyn Fn(Vec<u8>),
        stderr: dyn Fn(Vec<u8>),
        exit: dyn Fn(i32),
    ) -> dyn Process;
}

#[wasm_bus(format = "bincode")]
pub trait Process {
    fn stdin(&self, data: Vec<u8>) -> usize;
    fn close_stdin(&self);
    fn kill(&self);
    fn work(&self);
    fn flush(&self);
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
