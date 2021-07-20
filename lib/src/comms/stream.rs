use tokio::net::TcpStream;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
#[cfg(feature="websockets")]
use tokio_tungstenite::tungstenite::Message;
#[cfg(feature="websockets")]
use tokio_tungstenite::WebSocketStream;
use futures_util::stream;
use futures_util::StreamExt;
use futures_util::SinkExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::CommsError;

#[derive(Debug)]
pub enum Stream
{
    Tcp(TcpStream),
    #[cfg(feature="websockets")]
    WebSocket(WebSocketStream<TcpStream>)
}

#[derive(Debug, Clone, Copy)]
pub enum StreamProtocol
{
    Tcp,
    #[cfg(feature="websockets")]
    WebSocket,
}

impl StreamProtocol
{
    pub fn make_url(&self, domain: Option<String>) -> Result<url::Url, url::ParseError>
    {
        let input = match self {
            StreamProtocol::Tcp => match domain {
                Some(a) => format!("tcp://{}/", a),
                None => "tcp://localhost/".to_string()
            },
            #[cfg(feature="websockets")]
            StreamProtocol::WebSocket => match domain {
                Some(a) => format!("ws://{}/", a),
                None => "ws://localhost/".to_string()
            },
        };
        url::Url::parse(input.as_str())
    }

    pub fn parse(url: &url::Url) -> Result<StreamProtocol, CommsError>
    {
        let scheme = url.scheme().to_string().to_lowercase();
        match scheme.as_ref() {
            "tcp" => Ok(StreamProtocol::Tcp),
            "ws" => Ok(StreamProtocol::WebSocket),
            _ => Err(CommsError::UnsupportedProtocolError(scheme.clone())),
        }
    }
}

#[derive(Debug)]
pub enum StreamRx
{
    Tcp(OwnedReadHalf),
    #[cfg(feature="websockets")]
    WebSocket(stream::SplitStream<WebSocketStream<TcpStream>>)
}

#[derive(Debug)]
pub enum StreamTx
{
    Tcp(OwnedWriteHalf),
    #[cfg(feature="websockets")]
    WebSocket(stream::SplitSink<WebSocketStream<TcpStream>, Message>)
}

impl Stream
{
    pub fn split(self) -> (StreamRx, StreamTx) {
        match self {
            Stream::Tcp(a) => {
                let (rx, tx) = a.into_split();
                (StreamRx::Tcp(rx), StreamTx::Tcp(tx))
            },
            #[cfg(feature="websockets")]
            Stream::WebSocket(a) => {
                let (tx, rx) = a.split();
                (StreamRx::WebSocket(rx), StreamTx::WebSocket(tx))
            }
        }
    }

    pub async fn upgrade_server(self, protocol: StreamProtocol) -> Result<Stream, CommsError> {
        let ret = match self {
            Stream::Tcp(a) => {
                match protocol {
                    StreamProtocol::Tcp => Stream::Tcp(a),
                    #[cfg(feature="websockets")]
                    StreamProtocol::WebSocket => Stream::WebSocket(tokio_tungstenite::accept_async(a).await?),
                }
            },
            #[cfg(feature="websockets")]
            Stream::WebSocket(a) => {
                match protocol {
                    StreamProtocol::Tcp => Stream::WebSocket(a),
                    #[cfg(feature="websockets")]
                    StreamProtocol::WebSocket => Stream::WebSocket(a),
                }
            }
        };
        Ok(ret)
    }

    pub async fn upgrade_client(self, protocol: StreamProtocol, url: url::Url) -> Result<Stream, CommsError> {
        let ret = match self {
            Stream::Tcp(a) => {
                match protocol {
                    StreamProtocol::Tcp => Stream::Tcp(a),
                    #[cfg(feature="websockets")]
                    StreamProtocol::WebSocket => {
                        let (stream, response) = tokio_tungstenite::client_async(url, a)
                            .await?;
                        if response.status().is_client_error() {
                            return Err(CommsError::WebSocketInternalError(format!("HTTP error while performing WebSocket handshack - status-code={}", response.status().as_u16())));
                        }
                        Stream::WebSocket(stream)
                    },
                }
            },
            #[cfg(feature="websockets")]
            Stream::WebSocket(a) => {
                match protocol {
                    StreamProtocol::Tcp => Stream::WebSocket(a),
                    #[cfg(feature="websockets")]
                    StreamProtocol::WebSocket => Stream::WebSocket(a),
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
            #[cfg(feature="websockets")]
            Stream::WebSocket(_) => StreamProtocol::WebSocket
        }
    }
}

impl StreamTx
{
    pub async fn write_8bit(&mut self, buf: Vec<u8>) -> Result<(), tokio::io::Error>
    {
        match self {
            StreamTx::Tcp(a) => {
                if buf.len() > u8::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u8::MAX)));
                }
                a.write_u8(buf.len() as u8).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="websockets")]
            StreamTx::WebSocket(_) => {
                self.write_32bit(buf).await?;
            },
        }
        Ok(())
    }

    pub async fn write_16bit(&mut self, buf: Vec<u8>) -> Result<(), tokio::io::Error>
    {
        match self {
            StreamTx::Tcp(a) => {
                if buf.len() > u16::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u16::MAX)));
                }
                a.write_u16(buf.len() as u16).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="websockets")]
            StreamTx::WebSocket(_) => {
                self.write_32bit(buf).await?;
            },
        }
        Ok(())
    }

    pub async fn write_32bit(&mut self, buf: Vec<u8>) -> Result<(), tokio::io::Error>
    {
        match self {
            StreamTx::Tcp(a) => {
                if buf.len() > u32::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u32::MAX)));
                }
                a.write_u32(buf.len() as u32).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="websockets")]
            StreamTx::WebSocket(a) => {
                match a.feed(Message::binary(buf)).await {
                    Ok(a) => a,
                    Err(err) => {
                        return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to feed data into websocket - {}", err.to_string())));
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
            #[cfg(feature="websockets")]
            StreamRx::WebSocket(_) => {
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
            #[cfg(feature="websockets")]
            StreamRx::WebSocket(_) => {
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
            #[cfg(feature="websockets")]
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
        };
        Ok(ret)
    }
}