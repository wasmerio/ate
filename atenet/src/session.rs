use std::net::SocketAddr;
use ate::prelude::*;
use ate::comms::*;
use tokera::model::PortCommand;
use tokera::model::PortResponse;
use tokera::model::SwitchHello;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::port::*;

pub struct Session {
    pub rx: Box<dyn StreamReader + Send + Sync + 'static>,
    pub tx: Upstream,
    pub hello: HelloMetadata,
    pub hello_switch: SwitchHello,
    pub sock_addr: SocketAddr,
    pub wire_encryption: Option<EncryptKey>,
    pub port: Port,
}

impl Session
{
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        // Wait for commands to come in and then process them
        let wire_encryption = self.wire_encryption.clone();
        let mut total_read = 0u64;
        loop {
            let ret = self.port.poll();
            if ret.len() > 0 {
                self.send_response(ret).await;
            }
            let cmd = self.rx.read_buf_with_header(&wire_encryption, &mut total_read).await?;
            trace!("port read (len={})", cmd.len());

            let action: PortCommand = bincode::deserialize(&cmd[..])?;
            trace!("port cmd ({})", action);

            self.port.process(action);
        }
        Ok(())
    }

    async fn send_response(&mut self, ret: Vec<PortResponse>) {
        for ret in ret {
            match bincode::serialize(&ret) {
                Ok(ret) => {
                    let _=  self.tx.outbox.send(&ret[..]).await;
                }
                Err(err) => {
                    trace!("tx serialize failed - {}", err);
                }
            }            
        }
    }
}