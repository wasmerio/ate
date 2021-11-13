#![allow(dead_code)]
use std::io::Write;

use crate::web_command::WebCommand;
use crate::web_response::WebResponse;

use super::*;
use super::utils::*;

pub struct SocketBuilder {
    pub(crate) url: url::Url,
}

impl SocketBuilder {
    pub fn new(url: url::Url) -> SocketBuilder {
        SocketBuilder {
            url
        }
    }

    pub fn open(self) -> Result<WebSocket, std::io::Error> {
        let url = self.url.to_string();

        let submit = WebCommand::WebSocketVersion1 {
            url,
        };
        let mut submit = submit.serialize()?;
        submit += "\n";

        let mut file = std::fs::File::open("/dev/web")?;
        
        let _ = file.write_all(submit.as_bytes());

        let res = read_response(&mut file)?;
        match res {
            WebResponse::Error { msg } => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg.as_str()));
            },
            WebResponse::WebSocketVersion1 { .. } => {                
            },
            WebResponse::WebRequestVersion1 { .. } => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "server returned a web response instead of a web socket"));
            }
        };

        Ok(
            WebSocket {
                file
            }
        )
    }
}