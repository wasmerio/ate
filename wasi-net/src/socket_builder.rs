#![allow(dead_code)]
use std::io::Write;

use crate::web_command::WebCommand;

use super::*;

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

        let cmd = WebCommand::WebSocket {
            url,
        };
        let cmd = cmd.serialize()?;

        let mut file = std::fs::File::open("/dev/web")?;
        
        let submit = format!("{}\n", cmd);
        let _ = file.write_all(submit.as_bytes());

        Ok(
            WebSocket {
                file
            }
        )
    }
}