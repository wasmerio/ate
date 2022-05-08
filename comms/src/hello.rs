use std::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use ate_crypto::KeySize;
use ate_crypto::NodeId;
use ate_crypto::SerializationFormat;
use serde::{Deserialize, Serialize};

use super::protocol::MessageProtocolVersion;
use super::protocol::MessageProtocolApi;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HelloMetadata {
    pub client_id: NodeId,
    pub server_id: NodeId,
    pub path: String,
    pub encryption: Option<KeySize>,
    pub wire_format: SerializationFormat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SenderHello {
    pub id: NodeId,
    pub path: String,
    pub domain: String,
    pub key_size: Option<KeySize>,
    #[serde(default = "default_stream_protocol_version")]
    pub version: MessageProtocolVersion,
}

fn default_stream_protocol_version() -> MessageProtocolVersion {
    MessageProtocolVersion::V1
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ReceiverHello {
    pub id: NodeId,
    pub encryption: Option<KeySize>,
    pub wire_format: SerializationFormat,
    #[serde(default = "default_stream_protocol_version")]
    pub version: MessageProtocolVersion,
}

pub async fn mesh_hello_exchange_sender(
    stream_rx: Box<dyn AsyncRead + Send + Sync + Unpin + 'static>,
    stream_tx: Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>,
    client_id: NodeId,
    hello_path: String,
    domain: String,
    key_size: Option<KeySize>,
) -> tokio::io::Result<(
    Box<dyn MessageProtocolApi + Send + Sync + 'static>,
    HelloMetadata
)> {
    // Send over the hello message and wait for a response
    trace!("client sending hello");
    let hello_client = SenderHello {
        id: client_id,
        path: hello_path.clone(),
        domain,
        key_size,
        version: MessageProtocolVersion::default(),
    };
    let hello_client_bytes = serde_json::to_vec(&hello_client)?;
    let mut proto = MessageProtocolVersion::V1.create(
        Some(stream_rx),
        Some(stream_tx)
    );
    proto
        .write_with_fixed_16bit_header(&hello_client_bytes[..], false)
        .await?;

    // Read the hello message from the other side
    let hello_server_bytes = proto.read_with_fixed_16bit_header().await?;
    trace!("client received hello from server");
    trace!("{}", String::from_utf8_lossy(&hello_server_bytes[..]));
    let hello_server: ReceiverHello = serde_json::from_slice(&hello_server_bytes[..])?;

    // Validate the encryption is strong enough
    if let Some(needed_size) = &key_size {
        match &hello_server.encryption {
            None => {
                return Err(io::Error::new(io::ErrorKind::ConnectionRefused, "the server encryption strength is too weak"));
            }
            Some(a) if *a < *needed_size => {
                return Err(io::Error::new(io::ErrorKind::ConnectionRefused, "the server encryption strength is too weak"));
            }
            _ => {}
        }
    }

    // Switch to the correct protocol version
    let version = hello_server.version.min(hello_client.version);
    proto = version.upgrade(proto);
    
    // Upgrade the key_size if the server is bigger
    trace!(
        "client encryption={}",
        match &hello_server.encryption {
            Some(a) => a.as_str(),
            None => "none",
        }
    );
    trace!("client wire_format={}", hello_server.wire_format);

    Ok((
        proto,
        HelloMetadata {
            client_id,
            server_id: hello_server.id,
            path: hello_path,
            encryption: hello_server.encryption,
            wire_format: hello_server.wire_format,
        }
    ))
}

pub async fn mesh_hello_exchange_receiver(
    stream_rx: Box<dyn AsyncRead + Send + Sync + Unpin + 'static>,
    stream_tx: Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>,
    server_id: NodeId,
    key_size: Option<KeySize>,
    wire_format: SerializationFormat,
) -> tokio::io::Result<(
    Box<dyn MessageProtocolApi + Send + Sync + 'static>,
    HelloMetadata
)>
{
    // Read the hello message from the other side
    let mut proto = MessageProtocolVersion::V1.create(
        Some(stream_rx),
        Some(stream_tx)
    );
    let hello_client_bytes = proto
        .read_with_fixed_16bit_header()
        .await?;
    trace!("server received hello from client");
    //trace!("server received hello from client: {}", String::from_utf8_lossy(&hello_client_bytes[..]));
    let hello_client: SenderHello = serde_json::from_slice(&hello_client_bytes[..])?;

    // Upgrade the key_size if the client is bigger
    let encryption = mesh_hello_upgrade_key(key_size, hello_client.key_size);

    // Send over the hello message and wait for a response
    trace!("server sending hello (wire_format={})", wire_format);
    let hello_server = ReceiverHello {
        id: server_id,
        encryption,
        wire_format,
        version: MessageProtocolVersion::default(),
    };
    let hello_server_bytes = serde_json::to_vec(&hello_server)?;
    proto
        .write_with_fixed_16bit_header(&hello_server_bytes[..], false)
        .await?;

    // Switch to the correct protocol version
    proto = hello_server.version
        .min(hello_client.version)
        .upgrade(proto);

    Ok((
        proto,
        HelloMetadata {
            client_id: hello_client.id,
            server_id,
            path: hello_client.path,
            encryption,
            wire_format,
        }
    ))
}

fn mesh_hello_upgrade_key(key1: Option<KeySize>, key2: Option<KeySize>) -> Option<KeySize> {
    // If both don't want encryption then who are we to argue about that?
    if key1.is_none() && key2.is_none() {
        return None;
    }

    // Wanting encryption takes priority over not wanting encyption
    let key1 = match key1 {
        Some(a) => a,
        None => {
            trace!("upgrading to {}bit shared secret", key2.unwrap());
            return key2;
        }
    };
    let key2 = match key2 {
        Some(a) => a,
        None => {
            trace!("upgrading to {}bit shared secret", key1);
            return Some(key1);
        }
    };

    // Upgrade the key_size if the client is bigger
    if key2 > key1 {
        trace!("upgrading to {}bit shared secret", key2);
        return Some(key2);
    }
    if key1 > key2 {
        trace!("upgrading to {}bit shared secret", key2);
        return Some(key1);
    }

    // They are identical
    return Some(key1);
}
