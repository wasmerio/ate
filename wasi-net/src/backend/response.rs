use bincode;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Error {
        msg: String,
    },
    WebSocketVersion1 {},
    WebRequestVersion1 {
        ok: bool,
        redirected: bool,
        status: u16,
        status_text: String,
        headers: Vec<(String, String)>,
        has_data: bool,
    },
    SpawnedProcessVersion1 {
        pid: u32,
    }
}

impl Response {
    pub fn serialize(&self) -> io::Result<String> {
        let ret = bincode::serialize(self).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to serialize into bincode bytes - {}", err),
            )
        })?;
        Ok(base64::encode(&ret[..]))
    }

    pub fn deserialize(input: &str) -> io::Result<Response> {
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
