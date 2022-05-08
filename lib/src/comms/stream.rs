use crate::crypto::InitializationVector;
use crate::engine::timeout as tokio_timeout;
use bytes::BytesMut;
use error_chain::bail;
use std::io;
use std::ops::DerefMut;
use std::collections::VecDeque;
use std::fs::File;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::net::SocketAddr;
use std::result::Result;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::io::Error as TError;
use std::io::ErrorKind as TErrorKind;
#[cfg(feature = "enable_full")]
use tokio::net::tcp::OwnedReadHalf;
#[cfg(feature = "enable_full")]
use tokio::net::tcp::OwnedWriteHalf;
#[cfg(feature = "enable_full")]
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::ReadBuf;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use wasm_bus_ws::prelude::RecvHalf as WasmRecvHalf;
use wasm_bus_ws::prelude::SendHalf as WasmSendHalf;
use wasm_bus_ws::prelude::WebSocket as WasmWebSocket;
use bytes::Bytes;

use crate::comms::PacketData;
use crate::crypto::EncryptKey;
use super::helper::setup_tcp_stream;

pub use ate_comms::StreamRx;
pub use ate_comms::StreamTx;
pub use ate_comms::MessageProtocolVersion;
pub use ate_comms::StreamClient;
pub use ate_comms::StreamSecurity;
pub use ate_comms::Dns;

#[cfg(feature = "enable_server")]
use {
    hyper_tungstenite::hyper::upgrade::Upgraded as HyperUpgraded,
    hyper_tungstenite::tungstenite::Error as HyperError,
    hyper_tungstenite::tungstenite::Message as HyperMessage,
    hyper_tungstenite::WebSocketStream as HyperWebSocket,
};

#[cfg(feature = "enable_full")]
use {
    tokio::io::{AsyncReadExt, AsyncWriteExt},
    tokio_tungstenite::{tungstenite::Message, WebSocketStream},
};

use crate::error::*;

use super::NodeId;
use ate_comms::MessageProtocolApi;

#[derive(Debug, Clone, Copy)]
pub enum StreamProtocol {
    Tcp,
    WebSocket,
    SecureWebSocket,
}

impl std::str::FromStr for StreamProtocol {
    type Err = CommsError;

    fn from_str(s: &str) -> Result<StreamProtocol, CommsError> {
        let ret = match s {
            "tcp" => StreamProtocol::Tcp,
            "ws" => StreamProtocol::WebSocket,
            "wss" => StreamProtocol::SecureWebSocket,
            _ => {
                bail!(CommsErrorKind::UnsupportedProtocolError(s.to_string()));
            }
        };
        Ok(ret)
    }
}

impl StreamProtocol {
    pub fn to_scheme(&self) -> String {
        let ret = match self {
            StreamProtocol::Tcp => "tcp",
            StreamProtocol::WebSocket => "ws",
            StreamProtocol::SecureWebSocket => "wss",
        };
        ret.to_string()
    }

    pub fn to_string(&self) -> String {
        self.to_scheme()
    }

    pub fn default_port(&self) -> u16 {
        match self {
            StreamProtocol::Tcp => 5000,
            StreamProtocol::WebSocket => 80,
            StreamProtocol::SecureWebSocket => 443,
        }
    }

    pub fn is_tcp(&self) -> bool {
        match self {
            StreamProtocol::Tcp => true,
            StreamProtocol::WebSocket => false,
            StreamProtocol::SecureWebSocket => false,
        }
    }

    pub fn is_web_socket(&self) -> bool {
        match self {
            StreamProtocol::Tcp => false,
            StreamProtocol::WebSocket => true,
            StreamProtocol::SecureWebSocket => true,
        }
    }
}

impl std::fmt::Display for StreamProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_scheme())
    }
}

impl StreamProtocol {
    pub fn make_url(
        &self,
        domain: String,
        port: u16,
        path: String,
    ) -> Result<url::Url, url::ParseError> {
        let scheme = self.to_scheme();
        let input = match port {
            a if a == self.default_port() => match path.starts_with("/") {
                true => format!("{}://{}:{}{}", scheme, domain, port, path),
                false => format!("{}://{}:{}/{}", scheme, domain, port, path),
            },
            _ => match path.starts_with("/") {
                true => format!("{}://{}{}", scheme, domain, path),
                false => format!("{}://{}/{}", scheme, domain, path),
            },
        };
        url::Url::parse(input.as_str())
    }

    pub fn parse(url: &url::Url) -> Result<StreamProtocol, CommsError> {
        let scheme = url.scheme().to_string().to_lowercase();
        StreamProtocol::from_str(scheme.as_str())
    }

    pub async fn upgrade_client_and_split(&self, stream: TcpStream) -> Result<
        (
            Box<dyn AsyncRead + Send + Sync + Unpin + 'static>,
            Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>
        ), CommsError>
    {    
        // Setup the TCP stream
        setup_tcp_stream(&stream)?;

        // Convert the stream into rx/tx
        match self {
            StreamProtocol::Tcp => {
                let (rx, tx) = stream.into_split();
                Ok((
                    Box::new(rx),
                    Box::new(tx)
                ))
            },
            wire_protocol if self.is_web_socket() => {
                let port = match wire_protocol {
                    StreamProtocol::SecureWebSocket => 443,
                    _ => 80
                };
                let url = StreamProtocol::WebSocket.make_url(
                    "localhost".to_string(),
                    port,
                    "/".to_string(),
                )?;
                let mut request = tokio_tungstenite::tungstenite::http::Request::new(());
                *request.uri_mut() =
                    tokio_tungstenite::tungstenite::http::Uri::from_str(url.as_str())?;
                let (stream, response) = tokio_tungstenite::client_async(request, stream).await?;
                if response.status().is_client_error() {
                    bail!(CommsErrorKind::WebSocketInternalError(format!(
                        "HTTP error while performing WebSocket handshack - status-code={}",
                        response.status().as_u16()
                    )));
                }
                
                use futures_util::StreamExt;
                let (sink, stream) = stream.split();

                Ok((
                    Box::new(wasm_bus_ws::ws::RecvHalf::new(stream)),
                    Box::new(wasm_bus_ws::ws::SendHalf::new(sink))
                ))
            },
            wire_protocol => {
                bail!(CommsErrorKind::UnsupportedProtocolError(format!("the protocol isnt supported - {}", wire_protocol)));
            }
        }
    }

    pub async fn upgrade_server_and_split(&self, stream: TcpStream, timeout: Duration) -> Result<
        (
            Box<dyn AsyncRead + Send + Sync + Unpin + 'static>,
            Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>
        ), CommsError>
    {    
        // Setup the TCP stream
        setup_tcp_stream(&stream)?;

        // Convert the stream into rx/tx
        match self {
            StreamProtocol::Tcp => {
                let (rx, tx) = stream.into_split();
                Ok((
                    Box::new(rx),
                    Box::new(tx)
                ))
            },
            StreamProtocol::WebSocket |
            StreamProtocol::SecureWebSocket => {
                let wait = tokio_tungstenite::accept_async(stream);
                let socket = tokio_timeout(timeout, wait).await??;

                //use tokio::io::*;
                use futures_util::StreamExt;
                let (sink, stream) = socket.split();
                Ok((
                    Box::new(wasm_bus_ws::ws::RecvHalf::new(stream)),
                    Box::new(wasm_bus_ws::ws::SendHalf::new(sink))
                ))
            }
        }
    }
}
