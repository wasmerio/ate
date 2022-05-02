use std::net::SocketAddr;
use std::time::Duration;
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
            let (ret, wait_time) = self.port.poll();
            if ret.len() > 0 {
                self.send_response(ret).await;
            }

            let wait_time = Duration::max(wait_time, Duration::from_millis(5));
            let wait = tokio::time::sleep(wait_time);

            tokio::select! {
                _ = wait => { },
                cmd = self.rx.read_buf_with_header(&wire_encryption, &mut total_read) => {
                    let cmd = match cmd {
                        Ok(a) => a,
                        Err(err) => {
                            debug!("port read failed - {}", err);
                            break;
                        }
                    };
                    trace!("port read (len={})", cmd.len());

                    match bincode::deserialize::<PortCommand>(&cmd[..]) {
                        Ok(action) => {
                            trace!("port cmd ({})", action);

                            if let Err(err) = self.port.process(action) {
                                debug!("net-session-run - process-error: {}", err);
                            }
                        }
                        Err(err) => {
                            debug!("port failed deserialization - {}", err);
                        }
                    }
                },
                e = self.port.wake.changed() => {
                    if e.is_err() {
                        break;
                    }
                }
            }
        }
        let _ = self.tx.close().await;
        #[allow(unreachable_code)]
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