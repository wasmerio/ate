use async_trait::async_trait;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::SinkExt;
use futures_util::StreamExt;
use std::sync::Arc;
use std::io;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
#[allow(unused_imports)]
use tokio_tungstenite::{client_async_tls_with_config, connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::model::*;

#[derive(Debug)]
pub struct WebSocket {
    sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WebSocket {
    pub(crate) async fn new(url: &str) -> Result<Self, io::Error> {
        let request = url::Url::parse(url).unwrap();

        /*
        let domain = request
            .domain()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "URL does not have a domain component"))?;
        let port = request
            .port()
            .or_else(|| match request.scheme() {
                "wss" => Some(443),
                "ws" => Some(80),
                _ => None,
            })
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "URL does not have a port component"))?;

        let addr = format!("{}:{}", domain, port);
        let socket = TcpStream::connect(addr).await?;
        socket.set_nodelay(true)?;
        */

        let (ws_stream, _) = connect_async(request).await
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        //let (ws_stream, _) = client_async_tls_with_config(request, socket, None, None).await
        //    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let (sink, stream) = ws_stream.split();

        Ok(Self {
            sink,
            stream,
        })
    }

    pub fn split(self) -> (SendHalf, RecvHalf) {
        (
            SendHalf {
                sink: Arc::new(Mutex::new(self.sink)),
            },
            RecvHalf {
                stream: self.stream,
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct SendHalf {
    sink: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
}

impl SendHalf {
    pub async fn wait_till_opened(&self) -> SocketState {
        SocketState::Opened
    }

    pub async fn close(&self) -> io::Result<()> {
        let mut sink = self.sink.lock().await;
        let _ = sink.flush().await;
        sink.close().await
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(())
    }

    pub async fn send(&self, data: Vec<u8>) -> io::Result<usize> {
        let data_len = data.len();
        let mut sink = self.sink.lock().await;
        sink
            .send(Message::binary(data))
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(data_len)
    }

    pub fn blocking_send(&mut self, data: Vec<u8>) -> io::Result<usize> {
        let data_len = data.len();
        let sink = self.sink.clone();
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut sink = sink.lock().await;
                sink.send(Message::binary(data))
                    .await
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                Ok(data_len)
            })
        })
    }
}

#[derive(Debug)]
pub struct RecvHalf {
    stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        match self.stream.next().await {
            Some(Ok(Message::Binary(msg))) => {
                Some(msg)
            }
            Some(a) => {
                debug!("received invalid msg: {:?}", a);
                None
            }
            None => None
        }
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        let fut = self.stream.next();
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                match fut.await {
                    Some(Ok(Message::Binary(msg))) => {
                        Some(msg)
                    }
                    Some(a) => {
                        debug!("received invalid msg: {:?}", a);
                        None
                    }
                    None => None,
                }
            })
        })
    }
}