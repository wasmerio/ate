use async_trait::async_trait;

use crate::{crypto::EncryptKey, comms::stream::StreamTxInner, comms::stream::StreamRxInner};

#[async_trait]
pub trait MessageProtocolApi
where Self: std::fmt::Debug + Send + Sync
{
    async fn write_with_fixed_16bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error>;   

    async fn write_with_fixed_32bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error>;

    async fn send(
        &mut self,
        tx: &mut StreamTxInner,
        wire_encryption: &Option<EncryptKey>,
        data: &[u8],
    ) -> Result<u64, tokio::io::Error>;

    async fn read_with_fixed_16bit_header(
        &mut self,
        rx: &mut StreamRxInner,
    ) -> Result<Vec<u8>, tokio::io::Error>;

    async fn read_with_fixed_32bit_header(
        &mut self,
        rx: &mut StreamRxInner,
    ) -> Result<Vec<u8>, tokio::io::Error>;

    async fn read_buf_with_header(
        &mut self,
        rx: &mut StreamRxInner,
        wire_encryption: &Option<EncryptKey>,
        total_read: &mut u64
    ) -> std::io::Result<Vec<u8>>;

    async fn send_close(
        &mut self,
        tx: &mut StreamTxInner,
    ) -> std::io::Result<()>;
}