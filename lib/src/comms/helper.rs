#[allow(unused_imports)]
use log::{info, warn, debug};
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{net::{TcpStream}};
use tokio::net::tcp;
use bytes::Bytes;
use tokio::select;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use crate::spec::*;
use crate::crypto::*;
use crate::error::*;

use super::Packet;
use super::PacketData;
use super::PacketWithContext;

pub(super) fn setup_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    Ok(())
}

#[allow(unused_variables)]
pub(super) async fn process_inbox<M, C>(
    mut rx: tcp::OwnedReadHalf,
    reply_tx: mpsc::Sender<PacketData>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    sender: u64,
    context: Arc<C>,
    wire_format: SerializationFormat,
    wire_encryption: Option<EncryptKey>,
    terminate: tokio::sync::broadcast::Receiver<bool>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    loop
    {
        let buf = match wire_encryption {
            Some(key) => {
                // Read the initialization vector
                let iv_len = rx.read_u8().await? as usize;
                let mut iv_bytes = vec![0 as u8; iv_len];
                let n = rx.read_exact(&mut iv_bytes[0..iv_len]).await?;
                if n == 0 { break; }
                let iv = InitializationVector::from_bytes(iv_bytes);

                // Read the cipher text
                let cipher_len = rx.read_u32().await? as usize;
                let mut cipher_bytes = vec![0 as u8; cipher_len];
                let n = rx.read_exact(&mut cipher_bytes[0..cipher_len]).await?;
                if n == 0 { break; }

                // Decrypt the message
                key.decrypt(&iv, &cipher_bytes)?
            },
            None => {
                // Read the next message
                let buf_len = rx.read_u32().await? as usize;
                let mut buf = vec![0 as u8; buf_len];
                let n = rx.read_exact(&mut buf[0..buf_len]).await?;
                if n == 0 { break; }
                buf
            }
        };

        // Deserialize it
        let msg: M = wire_format.deserialize(&buf[..])?;
        let pck = Packet {
            msg,
        };
        
        // Process it
        let pck = PacketWithContext {
            data: PacketData {
                bytes: Bytes::from(buf),
                reply_here: Some(reply_tx.clone()),
                skip_here: Some(sender),
                wire_format,
            },
            context: Arc::clone(&context),
            packet: pck,
        };
         
        // Attempt to process the packet using the nodes inbox processing
        // thread (if its closed then we better close ourselves)
        match inbox.send(pck).await {
            Ok(a) => a,
            Err(mpsc::error::SendError(err)) => {
                break;
            },
        };
    }
    Ok(())
}

#[allow(unused_variables)]
pub(super) async fn process_outbox<M>(
    mut tx: tcp::OwnedWriteHalf,
    mut reply_rx: mpsc::Receiver<PacketData>,
    sender: u64,
    wire_encryption: Option<EncryptKey>,
    mut terminate: tokio::sync::broadcast::Receiver<bool>
) -> Result<mpsc::Receiver<PacketData>, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    loop
    {
        select! {
            buf = reply_rx.recv() =>
            {
                // Read the next message (and add the sender)
                if let Some(buf) = buf
                {
                    match wire_encryption {
                        Some(key) => {
                            // Encrypt the data
                            let enc = key.encrypt(&buf.bytes[..])?;
        
                            // Write the initialization vector
                            tx.write_u8(enc.iv.bytes.len() as u8).await?;
                            tx.write_all(&enc.iv.bytes[..]).await?;
        
                            // Write the cipher text
                            tx.write_u32(enc.data.len() as u32).await?;
                            tx.write_all(&enc.data[..]).await?;
                        },
                        None => {
                            // Write the bytes down the pipe
                            tx.write_u32(buf.bytes.len() as u32).await?;
                            tx.write_all(&buf.bytes).await?;
                        }
                    };
                } else {
                    return Ok(reply_rx);
                }
            },
            exit = terminate.recv() => {
                if exit? { return Ok(reply_rx); }
            },
        }
    }
}

#[allow(unused_variables)]
pub(super) async fn process_downcast<M>(
    tx: mpsc::Sender<PacketData>,
    mut outbox: broadcast::Receiver<PacketData>,
    sender: u64,
    mut terminate: tokio::sync::broadcast::Receiver<bool>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    loop
    {
        select! {
            pck = outbox.recv() => {
                let pck = pck?;
                if let Some(skip) = pck.skip_here {
                    if sender == skip {
                        continue;
                    }
                }
                tx.send(pck).await?;
            },
            exit = terminate.recv() => {
                if exit? { break; }
            },
        };
    }
    Ok(())
}