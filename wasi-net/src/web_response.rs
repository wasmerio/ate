use serde::{Serialize, Deserialize};
use bincode;
use std::io;

#[derive(Serialize, Deserialize, Debug)]
pub enum WebResponse
{
    Error {
        msg: String,
    },
    WebSocketVersion1 {
    },
    WebRequestVersion1 {
        ok: bool,
        redirected: bool,
        status: u16,
        status_text: String,
        headers: Vec<(String, String)>,
        has_data: bool,
    }
}

impl WebResponse
{
    pub fn serialize(&self) -> io::Result<String> {
        let ret = bincode::serialize(self)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("failed to serialize into bincode bytes - {}", err)))?;
        Ok(base64::encode(&ret[..]))
    }

    pub fn deserialize(input: &str) -> io::Result<WebResponse> {
        let input = base64::decode(input.trim())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("failed to decode base64 string - {}", err)))?;
        Ok(bincode::deserialize(&input[..])
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("failed to deserialize from bincode bytes - {}", err)))?)
    }
}