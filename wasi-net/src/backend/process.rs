use base64;
use bincode;
use serde::{Deserialize, Serialize};
use std::{io, process::ExitStatus};

use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageProcess {
    Kill,
    Exited(i32),
    Stdin(Vec<u8>),
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
}

impl MessageProcess {
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
