#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use std::pin::Pin;
use core::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::io;
use std::net::SocketAddr;

pub enum HyperStream
where Self: Send + Sync
{
    PlainTcp((TcpStream, SocketAddr)),
    Tls((TlsStream<TcpStream>, SocketAddr))
}

impl HyperStream
{
    pub fn remote_addr(&self) -> &SocketAddr
    {
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