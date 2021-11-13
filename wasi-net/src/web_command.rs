use serde::{Serialize, Deserialize};
use bincode;
use base64;
use std::io;

#[derive(Serialize, Deserialize, Debug)]
pub enum WebCommand {
    WebSocketVersion1 {
        url: String
    },
    WebRequestVersion1 {
        url: String,
        method: String,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    }
}

impl WebCommand
{
    pub fn serialize(&self) -> io::Result<String> {
        let ret = bincode::serialize(self)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("failed to serialize into bincode bytes - {}", err)))?;
        Ok(base64::encode(&ret[..]))
    }

    pub fn deserialize(input: &str) -> io::Result<WebCommand> {
        let input = base64::decode(input)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("failed to decode base64 string - {}", err)))?;
        Ok(bincode::deserialize(&input[..])
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("failed to deserialize from bincode bytes - {}", err)))?)
    }
}