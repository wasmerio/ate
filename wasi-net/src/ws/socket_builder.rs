#![allow(dead_code)]
use std::io::Write;

use super::*;
use crate::backend::utils::*;
use crate::backend::*;

pub struct SocketBuilder {
    pub(crate) url: url::Url,
}

impl SocketBuilder {
    pub fn new(url: url::Url) -> SocketBuilder {
        SocketBuilder { url }
    }

    pub fn open(self) -> Result<WebSocket, std::io::Error> {
        let url = self.url.to_string();

        let submit = Command::WebSocketVersion1 { url };
        let mut submit = submit.serialize()?;
        submit += "\n";

        let mut file = std::fs::File::open("/dev/web")?;

        let _ = file.write_all(submit.as_bytes());

        let res = read_response(&mut file)?;
        match res {
            Response::Error { msg } => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg.as_str()));
            }
            Response::WebSocketVersion1 { .. } => {}
            Response::WebRequestVersion1 { .. } => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "server returned a web response instead of a web socket",
                ));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "the socket does not support this response type",
                ));
            }
        };

        Ok(WebSocket { file })
    }
}
