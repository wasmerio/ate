use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use bytes::Bytes;
use async_trait::async_trait;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::SinkExt;
use futures_util::StreamExt;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio::io::AsyncWrite;
use tokio::io::AsyncRead;
use tokio::io::ReadBuf;
use tokio_tungstenite::{client_async_tls_with_config, tungstenite::protocol::Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::model::*;

pub(crate) async fn connect(url: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, io::Error> {
    let request = url::Url::parse(url).unwrap();

    let host = request
        .host()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "URL does not have a host component"))?;            
    let port = request
        .port()
        .or_else(|| match request.scheme() {
            "wss" => Some(443),
            "ws" => Some(80),
            _ => None,
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "URL does not have a port component"))?;

    let addr = format!("{}:{}", host, port);
    let socket = TcpStream::connect(addr).await?;
    socket.set_nodelay(true)?;

    let (ws_stream, _) = client_async_tls_with_config(request, socket, None, None).await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    let (sink, stream) = ws_stream.split();

    Ok(WebSocket::new(sink, stream))
}

#[derive(Debug)]
pub struct WebSocket<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    sink: SplitSink<WebSocketStream<S>, Message>,
    stream: SplitStream<WebSocketStream<S>>,
}

impl<S> WebSocket<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    pub fn new(sink: SplitSink<WebSocketStream<S>, Message>, stream: SplitStream<WebSocketStream<S>>) -> Self {
        Self {
            sink,
            stream
        }
    }

    pub fn split(self) -> (SendHalf<S>, RecvHalf<S>) {
        (
            SendHalf::new(self.sink),
            RecvHalf::new(self.stream)
        )
    }
}

#[derive(Debug)]
pub struct SendHalf<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    sink: SplitSink<WebSocketStream<S>, Message>,
}

impl<S> SendHalf<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    pub fn new(sink: SplitSink<WebSocketStream<S>, Message>) -> Self {
        Self {
            sink,
        }
    }

    pub async fn wait_till_opened(&self) -> SocketState {
        SocketState::Opened
    }

    pub async fn close(&mut self) -> io::Result<()> {
        let _ = self.sink.flush().await;
        self.sink.close().await
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(())
    }

    pub async fn send(&mut self, data: Vec<u8>) -> io::Result<usize> {
        let data_len = data.len();
        self.sink
            .send(Message::binary(data))
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(data_len)
    }

    pub fn blocking_send(&mut self, data: Vec<u8>) -> io::Result<usize> {
        wasm_bus::task::block_on(self.send(data))
    }
}

impl<S> AsyncWrite
for SendHalf<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>>
    {
        match self.sink.poll_ready_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(_)) => {
                match self.sink.start_send_unpin(Message::Binary(buf.to_vec())) {
                    Ok(_) => Poll::Ready(Ok(buf.len())),
                    Err(err) => {
                        return Poll::Ready(Err(
                            io::Error::new(io::ErrorKind::BrokenPipe, err.to_string())
                        ));
                    }
                }
            }
            Poll::Ready(Err(err)) => {
                return Poll::Ready(Err(
                    io::Error::new(io::ErrorKind::BrokenPipe, err.to_string())
                ));
            }
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>>
    {
        match self.sink.poll_flush_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(_)) => Poll::Ready(Ok(())),
            Poll::Ready(Err(err)) => {
                Poll::Ready(Err(
                    io::Error::new(io::ErrorKind::BrokenPipe, err.to_string())
                ))
            }
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>>
    {
        match self.sink.poll_flush_unpin(cx) {
            Poll::Pending => {
                return Poll::Pending;
            },
            _ => { }
        }
        match self.sink.poll_close_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(a)) => Poll::Ready(Ok(a)),
            Poll::Ready(Err(err)) => {
                Poll::Ready(Err(
                    io::Error::new(io::ErrorKind::Other, err.to_string())
                ))
            }
        }
    }
}

#[derive(Debug)]
pub struct RecvHalf<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    stream: SplitStream<WebSocketStream<S>>,
    buffer: Option<Bytes>,
}

impl<S> RecvHalf<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    pub fn new(stream: SplitStream<WebSocketStream<S>>) -> Self {
        Self {
            stream,
            buffer: None,
        }
    }

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

impl<S> AsyncRead
for RecvHalf<S>
where S: AsyncRead + AsyncWrite + Unpin
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some(stream) = self.buffer.take() {
            if stream.len() <= buf.remaining() {
                buf.put_slice(&stream[..]);
            } else {
                let end = buf.remaining();
                buf.put_slice(&stream[..end]);
                self.buffer.replace(stream.slice(end..));
            }
            return Poll::Ready(Ok(()));
        }
        match self.stream.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                Poll::Ready(Err(tokio::io::Error::new(
                    tokio::io::ErrorKind::BrokenPipe,
                    format!("Failed to receive data from websocket"),
                )))
            },
            Poll::Ready(Some(Err(err))) => {
                Poll::Ready(Err(tokio::io::Error::new(
                    tokio::io::ErrorKind::BrokenPipe,
                    format!(
                        "Failed to receive data from websocket - {}",
                        err.to_string()
                    ),
                )))
            },
            Poll::Ready(Some(Ok(Message::Binary(stream)))) => {
                if stream.len() <= buf.remaining() {
                    buf.put_slice(&stream[..]);
                } else {
                    let end = buf.remaining();
                    buf.put_slice(&stream[..end]);
                    self.buffer.replace(Bytes::from(stream).slice(end..));
                }
                Poll::Ready(Ok(()))
            },
            Poll::Ready(Some(Ok(Message::Close(_)))) => {
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "web socket connection has closed"
                )))
            },
            Poll::Ready(Some(Ok(_))) => {
                Poll::Ready(Err(tokio::io::Error::new(
                    tokio::io::ErrorKind::BrokenPipe,
                    format!("Failed to receive data from websocket as the message was the wrong type")
                )))
            },
        }
    }
}
