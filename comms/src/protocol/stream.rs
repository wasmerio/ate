use std::io;
use ate_crypto::EncryptKey;
use async_trait::async_trait;

use super::MessageProtocolApi;
use super::StreamReadable;
use super::StreamWritable;

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

#[async_trait]
impl StreamReadable
for StreamRx
{
    async fn read(&mut self) -> io::Result<Vec<u8>>
    {
        StreamRx::read(&mut self).await
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

#[async_trait]
impl StreamWritable
for StreamTx
{
    async fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        StreamTx::write(&mut self, data).await
    }
    
    async fn flush(&mut self) -> io::Result<()> {
        StreamTx::flush(&mut self).await
    }

    async fn close(&mut self) -> io::Result<()> {
        StreamTx::close(&mut self).await
    }

    fn wire_encryption(&self) -> Option<EncryptKey> {
        StreamTx::wire_encryption(&self)
    }
}
