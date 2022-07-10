use std::io;
#[allow(unused_imports)]
use std::ops::DerefMut;
use wasmer_bus_ws::prelude::*;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use ate_crypto::KeySize;
use ate_crypto::NodeId;

use crate::HelloMetadata;
use super::protocol::StreamRx;
use super::protocol::StreamTx;
use super::CertificateValidation;
use super::certificate_validation::GLOBAL_CERTIFICATES;

pub struct StreamClient
{
    rx: StreamRx,
    tx: StreamTx,
    hello: HelloMetadata,
}

pub use super::security::StreamSecurity;

impl StreamClient
{
    pub async fn connect(connect_url: url::Url, path: &str, security: StreamSecurity, #[allow(unused)] dns_server: Option<String>, #[allow(unused)] dns_sec: bool) -> Result<Self, Box<dyn std::error::Error>>
    {
        let https = match connect_url.scheme() {
            "https" => true,
            "wss" => true,
            _ => false,
        };

        let host = connect_url
            .host()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "URL does not have a host component"))?;
        let domain = match &host {
                url::Host::Domain(a) => Some(*a),
                url::Host::Ipv4(ip) if ip.is_loopback() => Some("localhost"),
                url::Host::Ipv6(ip) if ip.is_loopback() => Some("localhost"),
                _ => None
            }
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "URL does not have a domain component"))?;

        #[allow(unused_variables)]
        let mut validation = {
            let mut certs = Vec::new();

            #[cfg(feature = "dns")]
            #[cfg(not(target_family = "wasm"))]
            {
                let dns_server = dns_server.as_ref().map(|a| a.as_ref()).unwrap_or("8.8.8.8");
                let mut dns = crate::Dns::connect(dns_server, dns_sec).await;
                for cert in dns.dns_certs(domain).await {
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
        #[allow(unused_assignments)]
        if domain == "localhost" || security.quantum_encryption(https) == false {
            validation = CertificateValidation::AllowAll;
        }

        let socket = SocketBuilder::new(connect_url.clone())
            .open()
            .await?;
            
        let (tx, rx) = socket.split(); 
        let tx: Box<dyn AsyncWrite + Send + Sync + Unpin + 'static> = Box::new(tx);
        let rx: Box<dyn AsyncRead + Send + Sync + Unpin + 'static> = Box::new(rx);

        // We only encrypt if it actually has a certificate (otherwise
        // a simple man-in-the-middle could intercept anyway)
        let key_size = if security.quantum_encryption(https) == true {
            Some(KeySize::Bit192)
        } else {
            None
        };

        // Say hello
        let node_id = NodeId::generate_client_id();
        let (mut proto, hello_metadata) = super::hello::mesh_hello_exchange_sender(
            rx,
            tx,
            node_id,
            path.to_string(),
            domain.to_string(),
            key_size,
        )
        .await?;

        // If we are using wire encryption then exchange secrets
        #[cfg(feature = "quantum")]
        let ek = match hello_metadata.encryption {
            Some(key_size) => Some(
                super::key_exchange::mesh_key_exchange_sender(
                    proto.deref_mut(),
                    key_size,
                    validation,
                )
                .await?,
            ),
            None => None,
        };
        #[cfg(not(feature = "quantum"))]
        let ek = None;

        // Create the rx and tx message streams
        let (rx, tx) = proto.split(ek);
        Ok(
            Self {
                rx,
                tx,
                hello: hello_metadata,
            }
        )
    }

    pub fn split(self) -> (StreamRx, StreamTx)
    {
        (
            self.rx,
            self.tx,
        )
    }

    pub fn hello_metadata(&self) -> &HelloMetadata {
        &self.hello
    }
}
