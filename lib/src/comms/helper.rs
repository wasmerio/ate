#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};
#[cfg(feature="enable_tcp")]
use tokio::{net::{TcpStream}};
use bytes::Bytes;
#[allow(unused_imports)]
use tokio::io::{self};
use tokio::io::Error as TError;
use tokio::io::ErrorKind;
use async_trait::async_trait;
use std::net::SocketAddr;

use crate::spec::*;
use crate::crypto::*;
use crate::error::*;

use super::Packet;
use super::PacketData;
use super::PacketWithContext;
use super::StreamRx;

#[cfg(feature="enable_dns")]
type MeshConnectAddr = SocketAddr;
#[cfg(not(feature="enable_dns"))]
type MeshConnectAddr = crate::conf::MeshAddress;

#[async_trait]
pub(crate) trait InboxProcessor<M, C>
where Self: Send + Sync,
      M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    async fn process(&mut self, pck: PacketWithContext<M, C>) -> Result<(), CommsError>;

    async fn shutdown(&mut self, addr: MeshConnectAddr);
}

#[cfg(feature="enable_tcp")]
pub(super) fn setup_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    Ok(())
}

#[allow(unused_variables)]
pub(super) async fn process_inbox<M, C>(
    mut rx: StreamRx,
    mut inbox: Box<dyn InboxProcessor<M, C>>,
    sender: u64,
    sock_addr: MeshConnectAddr,
    context: Arc<C>,
    wire_format: SerializationFormat,
    wire_encryption: Option<EncryptKey>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    loop
    {
        let buf = async {
            match wire_encryption {
                Some(key) => {
                    // Read the initialization vector
                    let iv_bytes = rx.read_8bit().await?;
                    match iv_bytes.len() {
                        0 => Err(TError::new(ErrorKind::BrokenPipe, "iv_bytes-len is zero")),
                        _ => {
                            let iv = InitializationVector::from(iv_bytes);
                            // Read the cipher text and decrypt it
                            let cipher_bytes = rx.read_32bit().await?;
                            match cipher_bytes.len() {
                                0 => Err(TError::new(ErrorKind::BrokenPipe, "cipher_bytes-len is zero")),
                                _ => Ok(key.decrypt(&iv, &cipher_bytes))
                            }
                        }
                    }
                },
                None => {
                    // Read the next message
                    let buf = rx.read_32bit().await?;
                    match buf.len() {
                        0 => Err(TError::new(ErrorKind::BrokenPipe, "buf-len is zero")),
                        _ => Ok(buf)
                    }
                }
            }
        };
        let buf = buf.await?;
            
        // Deserialize it
        let msg: M = wire_format.deserialize(&buf[..])?;
        let pck = Packet {
            msg,
        };
        
        // Process it
        let pck = PacketWithContext {
            data: PacketData {
                bytes: Bytes::from(buf),
                wire_format,
            },
            context: Arc::clone(&context),
            packet: pck,
        };

        // Its time to process the packet
        let rcv = inbox.process(pck);
        match rcv.await {
            Ok(a) => a,
            Err(CommsError::Disconnected) => { break; }
            Err(CommsError::SendError(err)) => {
                warn!("inbox-err: {}", err);
                break;
            }
            Err(CommsError::ValidationError(errs)) => {
                for err in errs.iter() {
                    trace!("val-err: {}", err);
                }

                #[cfg(debug_assertions)]
                warn!("inbox-debug: {} validation errors", errs.len());
                #[cfg(not(debug_assertions))]
                debug!("inbox-debug: {} validation errors", errs.len());
                continue;
            }
            Err(err) => {
                warn!("inbox-error: {}", err.to_string());
                continue;
            }
        }
    }

    inbox.shutdown(sock_addr).await;
    Ok(())
}