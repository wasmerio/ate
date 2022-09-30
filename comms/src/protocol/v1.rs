use std::io;
use std::ops::DerefMut;
use derivative::*;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use async_trait::async_trait;
use ate_crypto::{EncryptKey, InitializationVector};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::MessageProtocolApi;
use super::StreamRx;
use super::StreamTx;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MessageProtocol
{
    #[derivative(Debug = "ignore")]
    rx: Option<Box<dyn AsyncRead + Send + Sync + Unpin + 'static>>,
    #[derivative(Debug = "ignore")]
    tx: Option<Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>>,
}

impl MessageProtocol
{
    pub(super) fn new(rx: Option<Box<dyn AsyncRead + Send + Sync + Unpin + 'static>>, tx: Option<Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>>) -> Self {
        Self {
            rx,
            tx
        }
    }

    fn tx_guard<'a>(&'a mut self) -> io::Result<&'a mut (dyn AsyncWrite + Send + Sync + Unpin + 'static)> {
        self.tx.as_deref_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "this protocol does not support writing"))
    }

    fn rx_guard<'a>(&'a mut self) -> io::Result<&'a mut (dyn AsyncRead + Send + Sync + Unpin + 'static)> {
        self.rx.as_deref_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "this protocol does not support reading"))
    }

    async fn write_with_8bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        let tx = self.tx_guard()?;
        if buf.len() > u8::MAX as usize {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                format!(
                    "Data is too big to write (len={}, max={})",
                    buf.len(),
                    u8::MAX
                ),
            ));
        }

        let len = buf.len() as u8;
        let len_buf = len.to_be_bytes();
        if len <= 63 {
            let concatenated = [&len_buf[..], &buf[..]].concat();
            tx.write_all(&concatenated[..]).await?;
            if delay_flush == false {
                tx.flush().await?;
            }
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = len_buf.len() as u64 + len as u64;
            tx.write_all(&len_buf[..]).await?;
            tx.write_all(&buf[..]).await?;
            if delay_flush == false {
                tx.flush().await?;
            }
            Ok(total_sent)
        }        
    }

    async fn write_with_16bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        let tx = self.tx_guard()?;
        if buf.len() > u16::MAX as usize {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                format!(
                    "Data is too big to write (len={}, max={})",
                    buf.len(),
                    u8::MAX
                ),
            ));
        }

        let len = buf.len() as u16;
        let len_buf = len.to_be_bytes();
        if len <= 62 {
            let concatenated = [&len_buf[..], &buf[..]].concat();
            tx.write_all(&concatenated[..]).await?;
            if delay_flush == false {
                tx.flush().await?;
            }
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = len_buf.len() as u64 + len as u64;
            tx.write_all(&len_buf[..]).await?;
            tx.write_all(&buf[..]).await?;
            if delay_flush == false {
                tx.flush().await?;
            }
            Ok(total_sent)
        }
    }

    async fn write_with_32bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        let tx = self.tx_guard()?;
        if buf.len() > u32::MAX as usize {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                format!(
                    "Data is too big to write (len={}, max={})",
                    buf.len(),
                    u8::MAX
                ),
            ));
        }

        let len = buf.len() as u32;
        let len_buf = len.to_be_bytes();
        if len <= 60 {
            let concatenated = [&len_buf[..], &buf[..]].concat();
            tx.write_all(&concatenated[..]).await?;
            if delay_flush == false {
                tx.flush().await?;
            }
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = len_buf.len() as u64 + len as u64;
            tx.write_all(&len_buf[..]).await?;
            tx.write_all(&buf[..]).await?;
            if delay_flush == false {
                tx.flush().await?;
            }
            Ok(total_sent)
        }
    }

    async fn read_u8(&mut self) -> Result<u8, tokio::io::Error> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf).await?;
        Ok(u8::from_be_bytes(buf))
    }

    async fn read_u16(&mut self) -> Result<u16, tokio::io::Error> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf).await?;
        Ok(u16::from_be_bytes(buf))
    }

    async fn read_u32(&mut self) -> Result<u32, tokio::io::Error> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf).await?;
        Ok(u32::from_be_bytes(buf))
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), tokio::io::Error> {
        let rx = self.rx_guard()?;
        rx.read_exact(buf).await?;
        Ok(())
    }

    async fn read_with_8bit_header(
        &mut self,        
    ) -> Result<Vec<u8>, tokio::io::Error>
    {
        let len = self.read_u8().await?;
        if len <= 0 {
            //trace!("stream_rx::8bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        //trace!("stream_rx::8bit-header(next_msg={} bytes)", len);
        let mut bytes = vec![0 as u8; len as usize];
        self.read_exact(&mut bytes).await?;
        Ok(bytes)
    }
}

#[async_trait]
impl MessageProtocolApi
for MessageProtocol
{
    async fn write_with_fixed_16bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> { 
        self.write_with_16bit_header(buf, delay_flush).await
    }

    async fn write_with_fixed_32bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {   
        self.write_with_32bit_header(buf, delay_flush).await
    }

    async fn send(
        &mut self,
        wire_encryption: &Option<EncryptKey>,
        data: &[u8],
    ) -> Result<u64, tokio::io::Error> {
        let mut total_sent = 0u64;
        match wire_encryption {
            Some(key) => {
                let enc = key.encrypt(data);
                total_sent += self.write_with_8bit_header(&enc.iv.bytes, true).await?;
                total_sent += self.write_with_32bit_header(&enc.data, false).await?;
            }
            None => {
                total_sent += self.write_with_32bit_header(data, false).await?;
            }
        };
        Ok(total_sent)
    }

    async fn read_with_fixed_16bit_header(
        &mut self,
    ) -> Result<Vec<u8>, tokio::io::Error>
    {
        let len = self.read_u16().await?;
        if len <= 0 {
            //trace!("stream_rx::16bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        //trace!("stream_rx::16bit-header(next_msg={} bytes)", len);
        let mut bytes = vec![0 as u8; len as usize];
        self.read_exact(&mut bytes).await?;
        Ok(bytes)
    }

    async fn read_with_fixed_32bit_header(
        &mut self,
    ) -> Result<Vec<u8>, tokio::io::Error>
    {
        let len = self.read_u32().await?;
        if len <= 0 {
            //trace!("stream_rx::32bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        //trace!("stream_rx::32bit-header(next_msg={} bytes)", len);
        let mut bytes = vec![0 as u8; len as usize];
        self.read_exact(&mut bytes).await?;
        Ok(bytes)
    }

    async fn read_buf_with_header(
        &mut self,
        wire_encryption: &Option<EncryptKey>,
        total_read: &mut u64
    ) -> std::io::Result<Vec<u8>>
    {
        match wire_encryption {
            Some(key) => {
                // Read the initialization vector
                let iv_bytes = self.read_with_8bit_header().await?;
                *total_read += 1u64;
                match iv_bytes.len() {
                    0 => Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "iv_bytes-len is zero")),
                    l => {
                        *total_read += l as u64;
                        let iv = InitializationVector::from(iv_bytes);

                        // Read the cipher text and decrypt it
                        let cipher_bytes = self.read_with_fixed_32bit_header().await?;
                        *total_read += 4u64;
                        match cipher_bytes.len() {
                            0 => Err(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                "cipher_bytes-len is zero",
                            )),
                            l => {
                                *total_read += l as u64;
                                Ok(key.decrypt(&iv, &cipher_bytes))
                            }
                        }
                    }
                }
            }
            None => {
                // Read the next message
                let buf = self.read_with_fixed_32bit_header().await?;
                *total_read += 4u64;
                match buf.len() {
                    0 => Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "buf-len is zero")),
                    l => {
                        *total_read += l as u64;
                        Ok(buf)
                    }
                }
            }
        }
    }

    async fn send_close(
        &mut self,
    ) -> std::io::Result<()> {
        Ok(())
    }

    async fn flush(
        &mut self,
    ) -> std::io::Result<()> {
        let tx = self.tx_guard()?;
        tx.flush().await
    }

    fn rx(&mut self) -> Option<&mut (dyn AsyncRead + Send + Sync + Unpin + 'static)> {
        self.rx.as_mut().map(|a| a.deref_mut())
    }

    fn tx(&mut self) -> Option<&mut (dyn AsyncWrite + Send + Sync + Unpin + 'static)> {
        self.tx.as_mut().map(|a| a.deref_mut())
    }

    fn take_rx(&mut self) -> Option<Box<dyn AsyncRead + Send + Sync + Unpin + 'static>> {
        self.rx.take()
    }

    fn take_tx(&mut self) -> Option<Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>> {
        self.tx.take()
    }

    fn split(&mut self, ek: Option<EncryptKey>) -> (StreamRx, StreamTx) {
        let rx = self.rx.take();
        let tx = self.tx.take();

        let rx = Box::new(Self::new(rx, None));
        let tx = Box::new(Self::new(None, tx));

        let rx = StreamRx::new(rx, ek.clone());
        let tx = StreamTx::new(tx, ek.clone());
        (rx, tx)
    }
}
