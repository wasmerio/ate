use std::io;
use ate_crypto::EncryptKey;

use super::MessageProtocolApi;

pub trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync {}

impl<T> AsyncStream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync
{}

impl std::fmt::Debug for dyn AsyncStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("async-stream")
    }
}

#[derive(Debug)]
pub struct StreamRx {
    proto: Box<dyn MessageProtocolApi + Send + Sync + 'static>,
    ek: Option<EncryptKey>,
}

impl StreamRx
{
    pub(crate) fn new(proto: Box<dyn MessageProtocolApi + Send + Sync + 'static>, ek: Option<EncryptKey>) -> Self {
        Self {
            proto,
            ek
        }
    }
    
    pub async fn read(&mut self) -> io::Result<Vec<u8>>
    {
        let mut total_read = 0u64;
        self.proto.read_buf_with_header(&self.ek, &mut total_read).await
    }
}

#[derive(Debug)]
pub struct StreamTx {
    proto: Box<dyn MessageProtocolApi + Send + Sync + 'static>,
    ek: Option<EncryptKey>,
}

impl StreamTx
{
    pub(crate) fn new(proto: Box<dyn MessageProtocolApi + Send + Sync + 'static>, ek: Option<EncryptKey>) -> Self {
        Self {
            proto,
            ek
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> io::Result<usize>
    {
        self.proto.send(&self.ek, data).await
            .map(|a| a as usize)
    }

    pub async fn flush(&mut self) -> io::Result<()> {
        self.proto.flush().await
    }

    pub async fn close(&mut self) -> io::Result<()> {
        self.proto.send_close().await
    }

    pub fn wire_encryption(&self) -> Option<EncryptKey> {
        self.ek.clone()
    }
}
