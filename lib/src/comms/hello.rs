#![allow(unused_imports)]
use log::{info, warn, debug};
use tokio::{net::{TcpStream}};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::*;
use serde::{Serialize, Deserialize};
use crate::crypto::KeySize;
use crate::spec::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello
{
    pub domain: Option<String>,
    pub key_size: Option<KeySize>,
    pub wire_format: Option<SerializationFormat>,
}

pub(super) async fn mesh_hello_exchange_sender(stream: &mut TcpStream, domain: Option<String>, mut key_size: Option<KeySize>) -> Result<(Option<KeySize>, SerializationFormat), CommsError>
{
    // Send over the hello message and wait for a response
    debug!("client sending hello");
    let hello_client = Hello {
        domain,
        key_size,
        wire_format: None,
    };
    let hello_client_bytes = serde_json::to_vec(&hello_client)?;
    stream.write_u16(hello_client_bytes.len() as u16).await?;
    stream.write_all(&hello_client_bytes[..]).await?;

    // Read the hello message from the other side
    let hello_server_bytes_len = stream.read_u16().await?;
    let mut hello_server_bytes = vec![0 as u8; hello_server_bytes_len as usize];
    stream.read_exact(&mut hello_server_bytes).await?;
    debug!("client received hello from server");
    let hello_server: Hello = serde_json::from_slice(&hello_server_bytes[..])?;

    // Upgrade the key_size if the server is bigger
    key_size = mesh_hello_upgrade_key(key_size, hello_server.key_size);
    let wire_format = match hello_server.wire_format {
        Some(a) => a,
        None => {
            debug!("server did not send wire format");
            return Err(CommsError::NoWireFormat);
        }
    };
    
    Ok((
        key_size,
        wire_format
    ))
}

pub(super) async fn mesh_hello_exchange_receiver(stream: &mut TcpStream, mut key_size: Option<KeySize>, wire_format: SerializationFormat) -> Result<Option<KeySize>, CommsError>
{
    // Read the hello message from the other side
    let hello_client_bytes_len = stream.read_u16().await?;
    let mut hello_client_bytes = vec![0 as u8; hello_client_bytes_len as usize];
    stream.read_exact(&mut hello_client_bytes).await?;
    debug!("server received hello from client");
    let hello_client: Hello = serde_json::from_slice(&hello_client_bytes[..])?;

    // Upgrade the key_size if the client is bigger
    key_size = mesh_hello_upgrade_key(key_size, hello_client.key_size);

    // Send over the hello message and wait for a response
    debug!("server sending hello");
    let hello_server = Hello {
        domain: None,
        key_size,
        wire_format: Some(wire_format),
    };
    let hello_server_bytes = serde_json::to_vec(&hello_server)?;
    stream.write_u16(hello_server_bytes.len() as u16).await?;
    stream.write_all(&hello_server_bytes[..]).await?;

    Ok(key_size)
}

fn mesh_hello_upgrade_key(key1: Option<KeySize>, key2: Option<KeySize>) -> Option<KeySize>
{
    // If both don't want encryption then who are we to argue about that?
    if key1.is_none() && key2.is_none() {
        return None;
    }

    // Wanting encryption takes priority over not wanting encyption
    let key1 = match key1 {
        Some(a) => a,
        None => {
            debug!("upgrading to {}bit shared secret", key2.unwrap());
            return key2;
        }
    };
    let key2 = match key2 {
        Some(a) => a,
        None => {
            debug!("upgrading to {}bit shared secret", key1);
            return Some(key1);
        }
    };

    // Upgrade the key_size if the client is bigger
    if key2 > key1 {
        debug!("upgrading to {}bit shared secret", key2);
        return Some(key2);
    }
    if key1 > key2 {
        debug!("upgrading to {}bit shared secret", key2);
        return Some(key1);
    }

    // They are identical
    return Some(key1);
}