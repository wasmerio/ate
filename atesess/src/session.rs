use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use ate::prelude::*;
use ate::comms::*;
use ate_files::prelude::FileAccessor;
use tokera::model::InstanceCommand;
use tokera::model::InstanceHello;
use tokera::model::ServiceInstance;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use atessh::term_lib;
use term_lib::bin_factory::CachedCompiledModules;
use term_lib::console::Console;
use tokio::sync::mpsc;
use term_lib::api::ConsoleRect;

use super::adapter::FileAccessorAdapter;
use super::handler::SessionHandler;

pub struct Session
{
    pub rx: StreamRx,
    pub tx: Upstream,
    pub hello: HelloMetadata,
    pub hello_instance: InstanceHello,
    pub sock_addr: SocketAddr,
    pub wire_encryption: Option<EncryptKey>,
    pub accessor: Arc<FileAccessor>,
    pub native_files: Arc<FileAccessor>,
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub service_instance: DaoMut<ServiceInstance>,
    pub compiled_modules: Arc<CachedCompiledModules>,
    pub compiler: term_lib::eval::Compiler,
}

impl Session
{
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        // Wait for commands to come in and then process them
        let wire_encryption = self.wire_encryption.clone();
        let mut total_read = 0u64;
        loop {
            let cmd = self.rx.read_buf(&wire_encryption, &mut total_read).await?;
            debug!("session read (len={})", cmd.len());

            let action: InstanceCommand = serde_json::from_slice(&cmd[..])?;
            debug!("session cmd ({})", action);

            match action {
                InstanceCommand::Shell => {
                    return self.shell().await;
                }
                InstanceCommand::WasmBus => {
                    self.wasm_bus().await?;
                }
            }
        }
    }

    pub async fn shell(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("new connection from {}", self.sock_addr);

        // Check the access code matches what was passed in
        if self.hello_instance.access_token.eq_ignore_ascii_case(self.service_instance.admin_token.as_str()) == false {
            warn!("access denied to {} from {}", self.hello_instance.chain, self.sock_addr);
            let err: CommsError = CommsErrorKind::FatalError("access denied".to_string()).into();
            return Err(err.into());
        }

        // Create the handler
        let (exit_tx, mut exit_rx) = mpsc::channel(1);
        let handler = SessionHandler {
            tx: AsyncMutex::new(self.tx),
            native_files: self.native_files,
            rect: self.rect.clone(),
            exit: exit_tx,
        };
        let handler = Arc::new(handler);

        // Create the console
        let prompt = (&self.service_instance.name[0..9]).to_string();
        let location = format!("wss://tokera.sh/?no_welcome&prompt={}", prompt);
        let user_agent = "noagent".to_string();
        let compiled_modules = self.compiled_modules.clone();
        let mut console = Console::new(
            location,
            user_agent,
            self.compiler,
            handler,
            None,
            Some(Box::new(FileAccessorAdapter::new(&self.accessor))),
            compiled_modules,
        );
        console.init().await;
        
        // Enter a processing loop
        let mut total_read = 0;
        loop {
            tokio::select! {
                data = self.rx.read_buf(&self.wire_encryption, &mut total_read) => {
                    match data {
                        Ok(data) => {
                            let data = String::from_utf8_lossy(&data[..]);
                            console.on_data(data.into()).await;
                        }
                        Err(err) => {
                            info!("exiting from session ({}) - {}", self.service_instance.name, err);
                            break;        
                        }
                    }
                },
                _ = exit_rx.recv() => {
                    info!("exiting from session ({})", self.service_instance.name);
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn wasm_bus(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}