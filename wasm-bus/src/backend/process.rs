use serde::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spawn {
    pub path: String,
    pub args: Vec<String>,
    pub current_dir: Option<String>,
    pub stdin_mode: StdioMode,
    pub stdout_mode: StdioMode,
    pub stderr_mode: StdioMode,
    pub pre_open: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutOfBand {
    DataStdin(Vec<u8>),
    CloseStdin,
    Kill,
    Work,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataStdout(pub Vec<u8>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataStderr(pub Vec<u8>);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StdioMode {
    Piped,
    Inherit,
    Null,
}
