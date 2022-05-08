use wasm_bus_tty::prelude::*;
use ate::comms::{StreamTx, StreamRx, StreamSecurity};
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::model::{InstanceCommand, InstanceHello, InstanceReply};

pub struct InstanceClient
{
    rx: StreamRx,
    tx: StreamTx,
}

impl InstanceClient
{
    pub const PATH_INST: &'static str = "/inst";

    pub async fn new(connect_url: url::Url) -> Result<Self, Box<dyn std::error::Error>>
    {
        Self::new_ext(connect_url, Self::PATH_INST, StreamSecurity::AnyEncryption).await
    }

    pub async fn new_ext(connect_url: url::Url, path: &str, security: StreamSecurity) -> Result<Self, Box<dyn std::error::Error>>
    {
        let port = ate_comms::StreamClient::connect(
            connect_url,
            path,
            security,
            Some("8.8.8.8".to_string()),
            false)
            .await?;

        let (rx, tx) = port.split();

        Ok(
            Self {
                rx,
                tx,
            }
        )
    }

    pub fn split(self) -> (StreamTx, StreamRx)
    {
        (
            self.tx,
            self.rx,
        )
    }

    pub async fn send_hello(&mut self, hello: InstanceHello) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_vec(&hello)?;
        self.send_data(data).await?;
        Ok(())
    }

    pub async fn send_cmd(&mut self, cmd: InstanceCommand) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_vec(&cmd)?;
        self.send_data(data).await?;
        Ok(())
    }

    pub async fn send_data(&mut self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.write(&data[..]).await?;
        Ok(())
    }

    pub async fn run_shell(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdin = Tty::stdin().await?;
        let mut stdout = Tty::stdout().await?;

        loop {
            tokio::select! {
                data = self.rx.read() => {
                    if let Ok(data) = data {
                        if data.len() <= 0 {
                            break;
                        }
                        stdout.write(data).await?;
                        stdout.flush().await?;
                    } else {
                        break;
                    }
                }
                data = stdin.read() => {
                    if let Some(data) = data {
                        self.tx.write(&data[..]).await?;
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn run_read(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = Tty::stdout().await?;
        let mut stderr = Tty::stderr().await?;
        loop {
            match self.rx.read().await {
                Ok(data) => {
                    if data.len() <= 0 {
                        break;
                    }

                    let reply: InstanceReply = bincode::deserialize(&data[..])?;
                    match reply {
                        InstanceReply::FeedBytes {
                            handle: _,
                            data,
                        } => {
                            stdout.write(data).await?;
                            stdout.write("\r\n".as_bytes().to_vec()).await?;
                            stdout.flush().await?;
                            break;
                        },
                        InstanceReply::Stdout { data } => {
                            stdout.write(data).await?;
                            stdout.write("\r\n".as_bytes().to_vec()).await?;
                            stdout.flush().await?;
                        },
                        InstanceReply::Stderr { data } => {
                            stderr.write(data).await?;
                            stderr.write("\r\n".as_bytes().to_vec()).await?;
                            stderr.flush().await?;
                        },
                        InstanceReply::Error {
                            handle: _,
                            error,
                        } => {
                            let error = format!("error: {}\r\n", error);
                            let mut stderr = Tty::stderr().await?;
                            stderr.write(error.into_bytes()).await?;
                            stderr.flush().await?;
                            break;
                        }
                        InstanceReply::Terminate {
                            handle: _,
                        } => {
                            break;
                        }
                        InstanceReply::Exit => {
                            break;
                        }
                    }
                }
                _ => {
                    break;
                }
            }
        }
        Ok(())
    }
}
