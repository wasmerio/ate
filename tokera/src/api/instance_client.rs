#[cfg(not(target_arch = "wasm32"))]
use ate_auth::prelude::conf_cmd;
use wasm_bus_ws::prelude::*;
use wasm_bus_tty::prelude::*;
use ate::{prelude::*, comms::{StreamTx, StreamRx}};
use ate::mesh::GLOBAL_CERTIFICATES;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::model::{InstanceCommand, InstanceHello};

pub struct InstanceClient
{
    rx: StreamRx,
    tx: StreamTx,
    ek: Option<EncryptKey>
}

impl InstanceClient
{
    pub async fn new(connect_url: url::Url) -> Result<Self, Box<dyn std::error::Error>>
    {
        let domain = connect_url.domain().clone().map(|a| a.to_string()).unwrap_or("localhost".to_string());

        let validation = {
            let mut certs = Vec::new();
            
            #[cfg(not(target_arch = "wasm32"))]
            {
                let test_registry = Registry::new(&conf_cmd()).await;
                for cert in test_registry.dns_certs(domain.as_str()).await.unwrap() {
                    certs.push(cert);
                }
            }
            for cert in GLOBAL_CERTIFICATES.read().unwrap().iter() {
                if certs.contains(cert) == false {
                    certs.push(cert.clone());
                }
            }
            if certs.len() > 0 {
                CertificateValidation::AllowedCertificates(certs)
            } else if domain == "localhost" {
                CertificateValidation::AllowAll
            } else {
                CertificateValidation::DenyAll
            }
        };

        let socket = SocketBuilder::new(connect_url)
            .open()
            .await?;
            
        let (tx, rx) = socket.split(); 
        let mut tx = StreamTx::WasmWebSocket(tx);
        let mut rx = StreamRx::WasmWebSocket(rx);
        
        // Say hello
        let node_id = NodeId::generate_client_id();
        let hello_metadata = ate::comms::hello::mesh_hello_exchange_sender(
            &mut rx,
            &mut tx,
            node_id,
            "/sess".to_string(),
            domain,
            Some(KeySize::Bit192),
        )
        .await?;

        // If we are using wire encryption then exchange secrets
        let ek = match hello_metadata.encryption {
            Some(key_size) => Some(
                ate::comms::key_exchange::mesh_key_exchange_sender(
                    &mut rx,
                    &mut tx,
                    key_size,
                    validation,
                )
                .await?,
            ),
            None => None,
        };
        
        Ok(
            Self {
                rx,
                tx,
                ek,
            }
        )
    }

    pub async fn send_hello(&mut self, hello: InstanceHello) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_vec(&hello)?;
        self.tx.send(&self.ek, &data[..]).await?;
        Ok(())
    }

    pub async fn send_cmd(&mut self, cmd: InstanceCommand) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_vec(&cmd)?;
        self.tx.send(&self.ek, &data[..]).await?;
        Ok(())
    }

    pub async fn run_shell(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdin = Tty::stdin().await?;
        let mut stdout = Tty::stdout().await?;

        let mut total_read = 0u64;
        loop {
            tokio::select! {
                data = self.rx.read_buf_with_header(&self.ek, &mut total_read) => {
                    if let Ok(data) = data {
                        stdout.write(data).await?;
                        stdout.flush().await?;
                    } else {
                        break;
                    }
                }
                data = stdin.read() => {
                    if let Some(data) = data {
                        self.tx.send(&self.ek, &data[..]).await?;
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}