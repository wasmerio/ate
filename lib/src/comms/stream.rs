#![allow(unused_imports)]
use log::{info, warn, debug};
#[cfg(feature="enable_tcp")]
use tokio::net::TcpStream;
#[cfg(feature="enable_tcp")]
use tokio::net::tcp::OwnedReadHalf;
#[cfg(feature="enable_tcp")]
use tokio::net::tcp::OwnedWriteHalf;
use std::str::FromStr;
use tokio::time::timeout as tokio_timeout;
use std::time::Duration;
use std::result::Result;

#[allow(unused_imports)]
#[cfg(feature="enable_tcp")]
#[cfg(feature="enable_ws")]
use
{
    tokio_tungstenite     :: { tungstenite::{ Message }, WebSocketStream    },
    tokio                 :: { io::{ AsyncReadExt, AsyncWriteExt }          },
    futures_util          :: { StreamExt, SinkExt, stream                   },
};

#[cfg(feature="enable_wasm")]
#[cfg(feature="enable_ws")]
use
{
    futures               :: { AsyncReadExt                         } ,
	wasm_bindgen::prelude :: { *                                    } ,
	wasm_bindgen_test     :: { *                                    } ,
	ws_stream_wasm        :: { *                                    } ,
    futures               :: { io::{ ReadHalf, WriteHalf }          } ,
    futures               :: { stream::{ StreamExt }, sink::SinkExt } ,
    async_io_stream       :: { IoStream                             } ,
    tokio_util            :: { codec::{ BytesCodec, Framed }        } ,
    futures_util          :: { stream                               } ,
};

use crate::error::CommsError;

#[derive(Debug, Clone, Copy)]
pub enum StreamProtocol
{
    #[cfg(feature="enable_tcp")]
    Tcp,
    #[cfg(feature="enable_ws")]
    WebSocket,
}

impl std::str::FromStr
for StreamProtocol
{
    type Err = CommsError;

    fn from_str(s: &str) -> Result<StreamProtocol, CommsError>
    {
        let ret = match s {
            #[cfg(feature="enable_tcp")]
            "tcp" => StreamProtocol::Tcp,
            #[cfg(feature="enable_ws")]
            "ws" => StreamProtocol::WebSocket,
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
            #[cfg(feature="enable_tcp")]
            StreamProtocol::Tcp => "tcp",
            #[cfg(feature="enable_ws")]
            StreamProtocol::WebSocket => "ws",
        };
        ret.to_string()
    }

    pub fn to_string(&self) -> String
    {
        self.to_scheme()
    }

    pub fn default_port(&self) -> u16 {
        match self {
            #[cfg(feature="enable_tcp")]
            StreamProtocol::Tcp => 5000,
            StreamProtocol::WebSocket => 80,
        }
    }
}

impl std::fmt::Display
for StreamProtocol
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_scheme())
    }
}

#[derive(Debug)]
pub enum Stream
{
    #[cfg(feature="enable_tcp")]
    Tcp(TcpStream),
    #[cfg(not(feature="enable_wasm"))]
    #[cfg(feature="enable_ws")]
    WebSocket(WebSocketStream<TcpStream>, StreamProtocol),
    #[cfg(feature="enable_wasm")]
    #[cfg(feature="enable_ws")]
    WebSocket(IoStream<WsStreamIo, Vec<u8>>, StreamProtocol),
}

impl StreamProtocol
{
    pub fn make_url(&self, domain: String, port: u16, path: String) -> Result<url::Url, url::ParseError>
    {
        let scheme = self.to_scheme();
        let input = match port {
            a if a == self.default_port() => match path.starts_with("/") {
                true => format!("{}://{}:{}{}", scheme, domain, port, path),
                false => format!("{}://{}:{}/{}", scheme, domain, port, path),
            },
            _ => match path.starts_with("/") {
                true => format!("{}://{}{}", scheme, domain, path),
                false => format!("{}://{}/{}", scheme, domain, path),
            }
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
    #[cfg(feature="enable_tcp")]
    Tcp(OwnedReadHalf),
    #[cfg(not(feature="enable_wasm"))]
    #[cfg(feature="enable_ws")]
    WebSocket(stream::SplitStream<WebSocketStream<TcpStream>>),
    #[cfg(feature="enable_wasm")]
    #[cfg(feature="enable_ws")]
    WebSocket(stream::SplitStream<Framed<IoStream<WsStreamIo, Vec<u8>>, BytesCodec>>),
}

#[derive(Debug)]
pub enum StreamTx
{
    #[cfg(feature="enable_tcp")]
    Tcp(OwnedWriteHalf),
    #[cfg(not(feature="enable_wasm"))]
    #[cfg(feature="enable_ws")]
    WebSocket(stream::SplitSink<WebSocketStream<TcpStream>, Message>),
    #[cfg(feature="enable_wasm")]
    #[cfg(feature="enable_ws")]
    WebSocket(stream::SplitSink<Framed<IoStream<WsStreamIo, Vec<u8>>, BytesCodec>, bytes::Bytes>),
}

impl Stream
{
    pub fn split(self) -> (StreamRx, StreamTx) {
        match self {
            #[cfg(feature="enable_tcp")]
            Stream::Tcp(a) => {
                let (rx, tx) = a.into_split();
                (StreamRx::Tcp(rx), StreamTx::Tcp(tx))
            },
            #[cfg(feature="enable_ws")]
            Stream::WebSocket(a, _) => {
                let (tx, rx) = Framed::new( a, BytesCodec::new() ).split();
                (StreamRx::WebSocket(rx), StreamTx::WebSocket(tx))
            },
        }
    }

    #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
    pub async fn upgrade_server(self, protocol: StreamProtocol, timeout: Duration) -> Result<Stream, CommsError> {
        debug!("tcp-protocol-upgrade: {}", protocol);

        let ret = match self {
            #[cfg(feature="enable_tcp")]
            Stream::Tcp(a) => {
                match protocol {
                    StreamProtocol::Tcp => {
                        Stream::Tcp(a)
                    },
                    #[cfg(feature="enable_ws")]
                    StreamProtocol::WebSocket => {
                        let wait = tokio_tungstenite::accept_async(a);
                        let socket = tokio_timeout(timeout, wait).await??;
                        Stream::WebSocket(socket, protocol)
                    },
                }
            },
            #[cfg(feature="enable_ws")]
            Stream::WebSocket(a, p) => {
                match protocol {
                    #[cfg(feature="enable_tcp")]
                    StreamProtocol::Tcp => {
                        Stream::WebSocket(a, p)
                    },
                    StreamProtocol::WebSocket => {
                        Stream::WebSocket(a, p)
                    },
                }
            },
        };

        Ok(ret)
    }

    #[allow(unused_variables)]
    pub async fn upgrade_client(self, protocol: StreamProtocol) -> Result<Stream, CommsError> {
        debug!("tcp-protocol-upgrade: {}", protocol);

        let ret = match self {
            #[cfg(feature="enable_tcp")]
            Stream::Tcp(a) => {
                match protocol {
                    StreamProtocol::Tcp => Stream::Tcp(a),
                    #[cfg(feature="enable_ws")]
                    StreamProtocol::WebSocket => {
                        let url = StreamProtocol::WebSocket.make_url("localhost".to_string(), 80, "/".to_string())?;
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
            #[cfg(feature="enable_ws")]
            Stream::WebSocket(a, p) => {
                match protocol {
                    #[cfg(feature="enable_tcp")]
                    StreamProtocol::Tcp => Stream::WebSocket(a, p),
                    StreamProtocol::WebSocket => Stream::WebSocket(a, p),
                }
            },
        };
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn protocol(&self) -> StreamProtocol
    {
        match self {
            #[cfg(feature="enable_tcp")]
            Stream::Tcp(_) => StreamProtocol::Tcp,
            #[cfg(feature="enable_ws")]
            Stream::WebSocket(_, p) => p.clone(),
        }
    }
}

impl StreamTx
{
    #[allow(unused_variables)]
    pub async fn write_8bit(&mut self, buf: Vec<u8>, delay_flush: bool) -> Result<(), tokio::io::Error>
    {
        match self {
            #[cfg(feature="enable_tcp")]
            StreamTx::Tcp(a) => {
                if buf.len() > u8::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u8::MAX)));
                }
                a.write_u8(buf.len() as u8).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="enable_ws")]
            StreamTx::WebSocket(_) => {
                self.write_32bit(buf, delay_flush).await?;
            },
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn write_16bit(&mut self, buf: Vec<u8>, delay_flush: bool) -> Result<(), tokio::io::Error>
    {
        match self {
            #[cfg(feature="enable_tcp")]
            StreamTx::Tcp(a) => {
                if buf.len() > u16::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u16::MAX)));
                }
                a.write_u16(buf.len() as u16).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(feature="enable_ws")]
            StreamTx::WebSocket(_) => {
                self.write_32bit(buf, delay_flush).await?;
            },
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn write_32bit(&mut self, buf: Vec<u8>, delay_flush: bool) -> Result<(), tokio::io::Error>
    {
        match self {
            #[cfg(feature="enable_tcp")]
            StreamTx::Tcp(a) => {
                if buf.len() > u32::MAX as usize {
                    return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Data is to big to write (len={}, max={})", buf.len(), u32::MAX)));
                }
                a.write_u32(buf.len() as u32).await?;
                a.write_all(&buf[..]).await?; 
            },
            #[cfg(not(feature="enable_wasm"))]
            #[cfg(feature="enable_ws")]
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
            #[cfg(feature="enable_wasm")]
            #[cfg(feature="enable_ws")]
            StreamTx::WebSocket(a) => {
                if delay_flush {
                    match a.feed(bytes::Bytes::from(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to feed data into websocket - {}", err.to_string())));
                        }
                    }
                } else {
                    match a.send(bytes::Bytes::from(buf)).await {
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
            #[cfg(feature="enable_tcp")]
            StreamRx::Tcp(a) => {
                let len = a.read_u8().await?;
                if len <= 0 { return Ok(vec![]); }
                let mut bytes = vec![0 as u8; len as usize];
                let n = a.read_exact(&mut bytes).await?;
                if n != (len as usize) { return Ok(vec![]); }
                bytes
            },
            #[cfg(feature="enable_ws")]
            StreamRx::WebSocket(_) => {
                self.read_32bit().await?
            },
        };
        Ok(ret)
    }

    pub async fn read_16bit(&mut self) -> Result<Vec<u8>, tokio::io::Error>
    {
        let ret = match self {
            #[cfg(feature="enable_tcp")]
            StreamRx::Tcp(a) => {
                let len = a.read_u16().await?;
                if len <= 0 { return Ok(vec![]); }
                let mut bytes = vec![0 as u8; len as usize];
                let n = a.read_exact(&mut bytes).await?;
                if n != (len as usize) { return Ok(vec![]); }
                bytes
            },
            #[cfg(feature="enable_ws")]
            StreamRx::WebSocket(_) => {
                self.read_32bit().await?
            },
        };
        Ok(ret)
    }

    pub async fn read_32bit(&mut self) -> Result<Vec<u8>, tokio::io::Error>
    {
        let ret = match self {
            #[cfg(feature="enable_tcp")]
            StreamRx::Tcp(a) => {
                let len = a.read_u32().await?;
                if len <= 0 { return Ok(vec![]); }
                let mut bytes = vec![0 as u8; len as usize];
                let n = a.read_exact(&mut bytes).await?;
                if n != (len as usize) { return Ok(vec![]); }
                bytes
            },
            #[cfg(not(feature="enable_wasm"))]
            #[cfg(feature="enable_ws")]
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
            #[cfg(feature="enable_wasm")]
            #[cfg(feature="enable_ws")]
            StreamRx::WebSocket(a) => {
                match a.next().await {
                    Some(msg) => {
                        match msg {
                            Ok(a) => a.to_vec(),
                            Err(err) => {
                                return Err(tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, format!("Failed to receive data from websocket - {}", err.to_string())));
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