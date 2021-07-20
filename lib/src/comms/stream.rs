#![allow(unused_imports)]
use hyper::upgrade::Upgraded;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
#[cfg(feature="ws")]
use tokio_tungstenite::tungstenite::Message;
#[cfg(feature="ws")]
use tokio_tungstenite::WebSocketStream;
use futures_util::stream;
use futures_util::StreamExt;
use futures_util::SinkExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::str::FromStr;

use crate::error::CommsError;

#[derive(Debug, Clone, Copy)]
pub enum StreamProtocol
{
    Tcp,
    #[cfg(feature="ws")]
    TcpWebSocket,
    #[cfg(feature="ws")]
    TcpWebSocketSecure,
    #[cfg(feature="http_ws")]
    HttpWebSocket,
    #[cfg(feature="http_ws")]
    HttpsWebSocket,
}

impl std::str::FromStr
for StreamProtocol
{
    type Err = CommsError;

    fn from_str(s: &str) -> Result<StreamProtocol, CommsError>
    {
        let ret = match s {
            "tcp" => StreamProtocol::Tcp,
            #[cfg(feature="ws")]
            "tcp-ws" => StreamProtocol::TcpWebSocket,
            #[cfg(feature="ws")]
            "tcp-wss" => StreamProtocol::TcpWebSocketSecure,
            #[cfg(feature="http_ws")]
            "ws" => StreamProtocol::HttpWebSocket,
            #[cfg(feature="http_ws")]
            "wss" => StreamProtocol::HttpsWebSocket,
            _ => {
                return Err(CommsError::UnsupportedProtocolError(s.to_string()));
            }
        };
        Ok(ret)
    }
}

impl StreamProtocol
{
    pub fn to_scheme(&self) -> String
    {
        let ret = match self {
            StreamProtocol::Tcp => "tcp",
            #[cfg(feature="ws")]
            StreamProtocol::TcpWebSocket => "tcp-ws",
            #[cfg(feature="ws")]
            StreamProtocol::TcpWebSocketSecure => "tcp-wss",
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpWebSocket => "ws",
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpsWebSocket => "wss",
        };
        ret.to_string()
    }

    pub fn to_standardized_scheme(&self) -> String
    {
        let ret = match self {
            StreamProtocol::Tcp => "tcp",
            #[cfg(feature="ws")]
            StreamProtocol::TcpWebSocket => "ws",
            #[cfg(feature="ws")]
            StreamProtocol::TcpWebSocketSecure => "wss",
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpWebSocket => "ws",
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpsWebSocket => "wss",
        };
        ret.to_string()
    }

    pub fn to_string(&self) -> String
    {
        self.to_scheme()
    }

    pub fn is_websocket(&self) -> bool {
        match self {
            #[cfg(feature="ws")]
            StreamProtocol::TcpWebSocket => true,
            #[cfg(feature="ws")]
            StreamProtocol::TcpWebSocketSecure => true,
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpWebSocket => true,
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpsWebSocket => true,
            _ => false
        }
    }

    pub fn is_http(&self) -> bool {
        match self {
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpWebSocket => true,
            #[cfg(feature="http_ws")]
            StreamProtocol::HttpsWebSocket => true,
            _ => false
        }
    }
}

#[derive(Debug)]
pub enum Stream
{
    Tcp(TcpStream),
    #[cfg(feature="ws")]
    WebSocket(WebSocketStream<TcpStream>, StreamProtocol),
    #[cfg(feature="http_ws")]
    WebSocketUpgraded(WebSocketStream<Upgraded>, StreamProtocol),
}

impl StreamProtocol
{
    pub fn make_url(&self, domain: Option<String>) -> Result<url::Url, url::ParseError>
    {
        let scheme = self.to_scheme();
        let input = match domain {
            Some(a) => format!("{}://{}/", scheme, a),
            None => format!("{}://localhost/", scheme)
        };
        url::Url::parse(input.as_str())
    }

    pub fn make_standardized_url(&self, domain: Option<String>) -> Result<url::Url, url::ParseError>
    {
        let scheme = self.to_standardized_scheme();
        let input = match domain {
            Some(a) => format!("{}://{}/", scheme, a),
            None => format!("{}://localhost/", scheme)
        };
        url::Url::parse(input.as_str())
    }

    pub fn parse(url: &url::Url) -> Result<StreamProtocol, CommsError>
    {
        let scheme = url.scheme().to_string().to_lowercase();
        StreamProtocol::from_str(scheme.as_str())
    }
}

#[derive(Debug)]
pub enum StreamRx
{
    Tcp(OwnedReadHalf),
    #[cfg(feature="ws")]
    WebSocket(stream::SplitStream<WebSocketStream<TcpStream>>),
    #[cfg(feature="http_ws")]
    WebSocketUpgraded(stream::SplitStream<WebSocketStream<Upgraded>>)
}

#[derive(Debug)]
pub enum StreamTx
{
    Tcp(OwnedWriteHalf),
    #[cfg(feature="ws")]
    WebSocket(stream::SplitSink<WebSocketStream<TcpStream>, Message>),
    #[cfg(feature="http_ws")]
    WebSocketUpgraded(stream::SplitSink<WebSocketStream<Upgraded>, Message>)
}

impl Stream
{
    pub fn split(self) -> (StreamRx, StreamTx) {
        match self {
            Stream::Tcp(a) => {
                let (rx, tx) = a.into_split();
                (StreamRx::Tcp(rx), StreamTx::Tcp(tx))
            },
            #[cfg(feature="ws")]
            Stream::WebSocket(a, _) => {
                let (tx, rx) = a.split();
                (StreamRx::WebSocket(rx), StreamTx::WebSocket(tx))
            },
            #[cfg(feature="http_ws")]
            Stream::WebSocketUpgraded(a, _) => {
                let (tx, rx) = a.split();
                (StreamRx::WebSocketUpgraded(rx), StreamTx::WebSocketUpgraded(tx))
            }
        }
    }

    pub async fn upgrade_server(self, protocol: StreamProtocol) -> Result<Stream, CommsError> {
        let ret = match self {
            Stream::Tcp(a) => {
                match protocol.is_websocket() {
                    false => Stream::Tcp(a),
                    #[cfg(feature="ws")]
                    true => Stream::WebSocket(tokio_tungstenite::accept_async(a).await?, protocol),
                }
            },
            #[cfg(feature="ws")]
            Stream::WebSocket(a, p) => {
                match protocol.is_websocket() {
                    false => Stream::WebSocket(a, p),
                    true => Stream::WebSocket(a, p),
                }
            },
            #[cfg(feature="http_ws")]
            Stream::WebSocketUpgraded(a, p) => {
                match protocol.is_websocket() {
                    false => Stream::WebSocketUpgraded(a, p),
                    true => Stream::WebSocketUpgraded(a, p),
                }
            }
        };
        Ok(ret)
    }

    #[allow(unused_variables)]
    pub async fn upgrade_client(self, protocol: StreamProtocol, url: url::Url) -> Result<Stream, CommsError> {
        let ret = match self {
            Stream::Tcp(a) => {
                match protocol.is_websocket() {
                    false => Stream::Tcp(a),
                    #[cfg(feature="ws")]
                    true => {
                        let mut request = tokio_tungstenite::tungstenite::http::Request::new(());
                        *request.uri_mut() = tokio_tungstenite::tungstenite::http::Uri::from_str(url.as_str())?;

                        let (stream, response) = tokio_tungstenite::client_async(request, a)
                            .await?;
                        if response.status().is_client_error() {
                            return Err(CommsError::WebSocketInternalError(format!("HTTP error while performing WebSocket handshack - status-code={}", response.status().as_u16())));
                        }
                        Stream::WebSocket(stream, protocol)
                    },
                }
            },
            #[cfg(feature="ws")]
            Stream::WebSocket(a, p) => {
                match protocol.is_websocket() {
                    false => Stream::WebSocket(a, p),
                    true => Stream::WebSocket(a, p),
                }
            },
            #[cfg(feature="http_ws")]
            Stream::WebSocketUpgraded(a, p) => {
                match protocol.is_websocket() {
                    false => Stream::WebSocketUpgraded(a, p),
                    true => Stream::WebSocketUpgraded(a, p),
                }
            }
        };
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn protocol(&self) -> StreamProtocol
    {
        match self {
            Stream::Tcp(_) => StreamProtocol::Tcp,
            #[cfg(feature="ws")]
            Stream::WebSocket(_, p) => p.clone(),
            #[cfg(feature="http_ws")]
            Stream::WebSocketUpgraded(_, p) => p.clone(),
        }
    }
}

impl StreamTx
{
    #[allow(unused_variables)]
    pub async fn write_8bit(&mut self, buf: Vec<u8>, delay_flush: bool) -> Result<(), tokio::io::Error>
    {
        match self {
            StreamTx::Tcp(a) => {
                if buf.len() > u8::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u8::MAX)));
                }
                a.write_u8(buf.len() as u8).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="ws")]
            StreamTx::WebSocket(_) => {
                self.write_32bit(buf, delay_flush).await?;
            },
            #[cfg(feature="http_ws")]
            StreamTx::WebSocketUpgraded(_) => {
                self.write_32bit(buf, delay_flush).await?;
            },
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn write_16bit(&mut self, buf: Vec<u8>, delay_flush: bool) -> Result<(), tokio::io::Error>
    {
        match self {
            StreamTx::Tcp(a) => {
                if buf.len() > u16::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u16::MAX)));
                }
                a.write_u16(buf.len() as u16).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="ws")]
            StreamTx::WebSocket(_) => {
                self.write_32bit(buf, delay_flush).await?;
            },
            #[cfg(feature="http_ws")]
            StreamTx::WebSocketUpgraded(_) => {
                self.write_32bit(buf, delay_flush).await?;
            },
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn write_32bit(&mut self, buf: Vec<u8>, delay_flush: bool) -> Result<(), tokio::io::Error>
    {
        match self {
            StreamTx::Tcp(a) => {
                if buf.len() > u32::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u32::MAX)));
                }
                a.write_u32(buf.len() as u32).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="ws")]
            StreamTx::WebSocket(a) => {
                if delay_flush {
                    match a.feed(Message::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to feed data into websocket - {}", err.to_string())));
                        }
                    }
                } else {
                    match a.send(Message::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to feed data into websocket - {}", err.to_string())));
                        }
                    }
                }
            },
            #[cfg(feature="http_ws")]
            StreamTx::WebSocketUpgraded(a) => {
                if delay_flush {
                    match a.feed(Message::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to feed data into websocket - {}", err.to_string())));
                        }
                    }
                } else {
                    match a.send(Message::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to feed data into websocket - {}", err.to_string())));
                        }
                    }
                }
            },
        }
        Ok(())
    }
}

impl StreamRx
{
    pub async fn read_8bit(&mut self) -> Result<Vec<u8>, tokio::io::Error>
    {
        let ret = match self {
            StreamRx::Tcp(a) => {
                let len = a.read_u8().await?;
                if len <= 0 { return Ok(vec![]); }
                let mut bytes = vec![0 as u8; len as usize];
                let n = a.read_exact(&mut bytes).await?;
                if n != (len as usize) { return Ok(vec![]); }
                bytes
            },
            #[cfg(feature="ws")]
            StreamRx::WebSocket(_) => {
                self.read_32bit().await?
            },
            #[cfg(feature="http_ws")]
            StreamRx::WebSocketUpgraded(_) => {
                self.read_32bit().await?
            },
        };
        Ok(ret)
    }

    pub async fn read_16bit(&mut self) -> Result<Vec<u8>, tokio::io::Error>
    {
        let ret = match self {
            StreamRx::Tcp(a) => {
                let len = a.read_u16().await?;
                if len <= 0 { return Ok(vec![]); }
                let mut bytes = vec![0 as u8; len as usize];
                let n = a.read_exact(&mut bytes).await?;
                if n != (len as usize) { return Ok(vec![]); }
                bytes
            },
            #[cfg(feature="ws")]
            StreamRx::WebSocket(_) => {
                self.read_32bit().await?
            },
            #[cfg(feature="http_ws")]
            StreamRx::WebSocketUpgraded(_) => {
                self.read_32bit().await?
            },
        };
        Ok(ret)
    }

    pub async fn read_32bit(&mut self) -> Result<Vec<u8>, tokio::io::Error>
    {
        let ret = match self {
            StreamRx::Tcp(a) => {
                let len = a.read_u32().await?;
                if len <= 0 { return Ok(vec![]); }
                let mut bytes = vec![0 as u8; len as usize];
                let n = a.read_exact(&mut bytes).await?;
                if n != (len as usize) { return Ok(vec![]); }
                bytes
            },
            #[cfg(feature="ws")]
            StreamRx::WebSocket(a) => {
                match a.next().await {
                    Some(a) => {
                        let msg = match a {
                            Ok(a) => a,
                            Err(err) => {
                                return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Failed to receive data from websocket - {}", err.to_string())));
                            }
                        };
                        match msg {
                            Message::Binary(a) => a,
                            _ => {
                                return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Failed to receive data from websocket as the message was the wrong type")));
                            }
                        }
                    },
                    None => {
                        return Err(tokio::io::Error::new(tokio::io::ErrorKind::BrokenPipe, format!("Failed to receive data from websocket")));
                    }
                }
            },
            #[cfg(feature="http_ws")]
            StreamRx::WebSocketUpgraded(a) => {
                match a.next().await {
                    Some(a) => {
                        let msg = match a {
                            Ok(a) => a,
                            Err(err) => {
                                return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Failed to receive data from websocket - {}", err.to_string())));
                            }
                        };
                        match msg {
                            Message::Binary(a) => a,
                            _ => {
                                return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Failed to receive data from websocket as the message was the wrong type")));
                            }
                        }
                    },
                    None => {
                        return Err(tokio::io::Error::new(tokio::io::ErrorKind::BrokenPipe, format!("Failed to receive data from websocket")));
                    }
                }
            },
        };
        Ok(ret)
    }
}