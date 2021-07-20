#[allow(unused_imports)]
use log::{info, warn, debug};
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{net::{TcpStream}};
use bytes::Bytes;
use tokio::select;
use tokio::io::{self};

use crate::spec::*;
use crate::crypto::*;
use crate::error::*;

use super::Packet;
use super::PacketData;
use super::PacketWithContext;
use super::BroadcastContext;
use super::BroadcastPacketData;
use super::StreamRx;
use super::StreamTx;

pub(super) fn setup_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    Ok(())
}

#[allow(unused_variables)]
pub(super) async fn process_inbox<M, C>(
    mut rx: StreamRx,
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
                let iv_bytes = rx.read_8bit().await?;
                if iv_bytes.len() == 0 { break; }
                let iv = InitializationVector::from_bytes(iv_bytes);

                // Read the cipher text
                let cipher_bytes = rx.read_32bit().await?;
                if cipher_bytes.len() == 0 { break; }

                // Decrypt the message
                key.decrypt(&iv, &cipher_bytes)?
            },
            None => {
                // Read the next message
                let buf = rx.read_32bit().await?;
                if buf.len() == 0 { break; }
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
    mut tx: StreamTx,
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
                            tx.write_8bit(enc.iv.bytes, true).await?;
                            
                            // Write the cipher text
                            tx.write_32bit(enc.data, false).await?;
                        },
                        None => {
                            // Write the bytes down the pipe
                            tx.write_32bit(buf.bytes.to_vec(), false).await?;
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
pub(super) async fn process_downcast<M, C>(
    tx: mpsc::Sender<PacketData>,
    mut outbox: broadcast::Receiver<BroadcastPacketData>,
    sender: u64,
    context: Arc<C>,
    mut terminate: tokio::sync::broadcast::Receiver<bool>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync + BroadcastContext,
{
    loop
    {
        select! {
            pck = outbox.recv() => {
                let pck = pck?;

                if let Some(broadcast_group) = context.broadcast_group() {
                    if let Some(packet_group) = pck.group {
                        if broadcast_group != packet_group {
                            continue;
                        }
                    }
                } else {
                    if pck.group.is_some() {
                        continue;
                    }
                }

                if let Some(skip) = pck.data.skip_here {
                    if sender == skip {
                        continue;
                    }
                }
                tx.send(pck.data).await?;
            },
            exit = terminate.recv() => {
                if exit? { break; }
            },
        };
    }
    Ok(())
}