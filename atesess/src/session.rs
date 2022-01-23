use std::net::SocketAddr;
use std::sync::Arc;
use ate::prelude::*;
use ate::comms::*;
use ate_files::prelude::FileAccessor;
use tokera::model::InstanceAction;
use tokera::model::InstanceCommand;
use tokera::model::InstanceHello;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

pub struct Session
{
    pub rx: StreamRx,
    pub tx: Upstream,
    pub hello: HelloMetadata,
    pub hello_instance: InstanceHello,
    pub sock_addr: SocketAddr,
    pub wire_encryption: Option<EncryptKey>,
    pub accessor: Arc<FileAccessor>,
}

impl Session
{
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        // Wait for commands to come in and then process them
        let mut total_read = 0u64;
        loop {
            let cmd = self.rx.read_buf(&self.wire_encryption, &mut total_read).await?;
            debug!("session read (len={})", cmd.len());

            let action: InstanceCommand = bincode::deserialize(&cmd[..])?;
            debug!("session cmd ({})", action);

            match action {
                InstanceCommand::Action(action) => {
                    self.action(action).await?;
                }
                InstanceCommand::Shell => {
                    self.shell().await?;
                }
                InstanceCommand::WasmBus => {
                    self.wasm_bus().await?;
                }
            }
        }
    }

    pub async fn action(&self, _action: InstanceAction) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub async fn shell(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub async fn wasm_bus(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}