use core::task::{Context, Poll};
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use hyper_tungstenite::WebSocketStream;
use hyper_tungstenite::tungstenite::Message;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::SinkExt;
use futures_util::StreamExt;
use bytes::Bytes;

pub enum HyperStream
where
    Self: Send + Sync,
{
    PlainTcp((TcpStream, SocketAddr)),
    Tls((TlsStream<TcpStream>, SocketAddr)),
}

impl HyperStream {
    pub fn remote_addr(&self) -> &SocketAddr {
        match self {
            HyperStream::PlainTcp((_, addr)) => addr,
            HyperStream::Tls((_, addr)) => addr,
        }
    }
}

impl AsyncRead for HyperStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            HyperStream::PlainTcp((a, _)) => Pin::new(a).poll_read(cx, buf),
            HyperStream::Tls((a, _)) => Pin::new(a).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for HyperStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            HyperStream::PlainTcp((a, _)) => Pin::new(a).poll_write(cx, buf),
            HyperStream::Tls((a, _)) => Pin::new(a).poll_write(cx, buf),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            HyperStream::PlainTcp((a, _)) => Pin::new(a).poll_write_vectored(cx, bufs),
            HyperStream::Tls((a, _)) => Pin::new(a).poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            HyperStream::PlainTcp((a, _)) => Pin::new(a).is_write_vectored(),
            HyperStream::Tls((a, _)) => Pin::new(a).is_write_vectored(),
        }
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            HyperStream::PlainTcp((a, _)) => Pin::new(a).poll_flush(cx),
            HyperStream::Tls((a, _)) => Pin::new(a).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            HyperStream::PlainTcp((a, _)) => Pin::new(a).poll_shutdown(cx),
            HyperStream::Tls((a, _)) => Pin::new(a).poll_shutdown(cx),
        }
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
