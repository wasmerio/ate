use base64;
use bincode;
use serde::{Deserialize, Serialize};
use std::io;

use super::StdioMode;

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    WebSocketVersion1 {
        url: String,
    },
    WebRequestVersion1 {
        url: String,
        method: String,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    },
    SpawnProcessVersion1 {
        path: String,
        args: Vec<String>,
        current_dir: Option<String>,
        stdin_mode: StdioMode,
        stdout_mode: StdioMode,
        stderr_mode: StdioMode,
        pre_open: Vec<String>,
    },
}

impl Command {
    pub fn serialize(&self) -> io::Result<String> {
        let ret = bincode::serialize(self).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to serialize into bincode bytes - {}", err),
            )
        })?;
        Ok(base64::encode(&ret[..]))
    }

    pub fn deserialize(input: &str) -> io::Result<Self> {
        let input = base64::decode(input.trim()).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to decode base64 string - {}", err),
            )
        })?;
        Ok(bincode::deserialize(&input[..]).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to deserialize from bincode bytes - {}", err),
            )
        })?)
    }
}
