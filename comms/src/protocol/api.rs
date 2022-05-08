use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::io::ReadBuf;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use async_trait::async_trait;
use ate_crypto::EncryptKey;

use super::StreamRx;
use super::StreamTx;

#[async_trait]
pub trait MessageProtocolApi
where Self: std::fmt::Debug + Send + Sync,
{
    async fn write_with_fixed_16bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error>;   

    async fn write_with_fixed_32bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error>;

    async fn send(
        &mut self,
        wire_encryption: &Option<EncryptKey>,
        data: &[u8],
    ) -> Result<u64, tokio::io::Error>;

    async fn read_with_fixed_16bit_header(
        &mut self,
    ) -> Result<Vec<u8>, tokio::io::Error>;

    async fn read_with_fixed_32bit_header(
        &mut self,
    ) -> Result<Vec<u8>, tokio::io::Error>;

    async fn read_buf_with_header(
        &mut self,
        wire_encryption: &Option<EncryptKey>,
        total_read: &mut u64
    ) -> std::io::Result<Vec<u8>>;

    async fn send_close(
        &mut self,
    ) -> std::io::Result<()>;

    async fn flush(
        &mut self,
    ) -> std::io::Result<()>;

    fn split(&mut self, ek: Option<EncryptKey>) -> (StreamRx, StreamTx);

    fn rx(&mut self) -> Option<&mut (dyn AsyncRead + Send + Sync + Unpin + 'static)>;

    fn tx(&mut self) -> Option<&mut (dyn AsyncWrite + Send + Sync + Unpin + 'static)>;

    fn take_rx(&mut self) -> Option<Box<dyn AsyncRead + Send + Sync + Unpin + 'static>>;

    fn take_tx(&mut self) -> Option<Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>>;
}

impl AsyncRead
for dyn MessageProtocolApi + Unpin + Send + Sync
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let rx = match self.rx() {
            Some(rx) => rx,
            None => {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Unsupported, "this stream does not support reading")));
            }
        };
        let rx = Pin::new(rx);
        rx.poll_read(cx, buf)
    }
}

impl AsyncWrite
for dyn MessageProtocolApi + Unpin + Send + Sync
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let tx = match self.tx() {
            Some(tx) => tx,
            None => {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Unsupported, "this stream does not support writing")));
            }
        };
        let tx = Pin::new(tx);
        tx.poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>> {
        let tx = match self.tx() {
            Some(tx) => tx,
            None => {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Unsupported, "this stream does not support writing")));
            }
        };
        let tx = Pin::new(tx);
        tx.poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>> {
        let tx = match self.tx() {
            Some(tx) => tx,
            None => {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Unsupported, "this stream does not support writing")));
            }
        };
        let tx = Pin::new(tx);
        tx.poll_shutdown(cx)
    }
}
