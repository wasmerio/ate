#![allow(unused_imports)]
use async_trait::async_trait;
use bytes::Bytes;
use crate::{crypto::{EncryptKey, InitializationVector}, comms::stream::StreamTxInner, comms::stream::StreamRxInner};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::api::MessageProtocolApi;

#[derive(Debug, Clone, Default)]
pub struct MessageProtocol
{
    buffer: Option<Bytes>,
}

impl MessageProtocol
{
    async fn write_with_8bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
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
            tx.write_all(&concatenated[..], delay_flush).await?;
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = len_buf.len() as u64 + len as u64;
            tx.write_all(&len_buf[..], true).await?;
            tx.write_all(&buf[..], delay_flush).await?;
            Ok(total_sent)
        }        
    }

    async fn write_with_16bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
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
            tx.write_all(&concatenated[..], delay_flush).await?;
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = len_buf.len() as u64 + len as u64;
            tx.write_all(&len_buf[..], true).await?;
            tx.write_all(&buf[..], delay_flush).await?;
            Ok(total_sent)
        }
    }

    async fn write_with_32bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
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
            tx.write_all(&concatenated[..], delay_flush).await?;
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = len_buf.len() as u64 + len as u64;
            tx.write_all(&len_buf[..], true).await?;
            tx.write_all(&buf[..], delay_flush).await?;
            Ok(total_sent)
        }
    }

    async fn read_u8(&mut self, rx: &mut StreamRxInner) -> Result<u8, tokio::io::Error> {
        let mut buf = [0u8; 1];
        self.read_exact(rx, &mut buf).await?;
        Ok(u8::from_be_bytes(buf))
    }

    async fn read_u16(&mut self, rx: &mut StreamRxInner) -> Result<u16, tokio::io::Error> {
        let mut buf = [0u8; 2];
        self.read_exact(rx, &mut buf).await?;
        Ok(u16::from_be_bytes(buf))
    }

    async fn read_u32(&mut self, rx: &mut StreamRxInner) -> Result<u32, tokio::io::Error> {
        let mut buf = [0u8; 4];
        self.read_exact(rx, &mut buf).await?;
        Ok(u32::from_be_bytes(buf))
    }

    async fn read_exact(&mut self, rx: &mut StreamRxInner, buf: &mut [u8]) -> Result<(), tokio::io::Error> {
        use bytes::Buf;

        let mut index = 0;
        while index < buf.len() {
            let left = buf.len() - index;

            // If we have any data then lets go!
            if let Some(staging) = self.buffer.as_mut() {
                if staging.has_remaining() {
                    let amount = staging.remaining().min(left);
                    let end = index + amount;
                    buf[index..end].copy_from_slice(&staging[..amount]);
                    staging.advance(amount);
                    index += amount;
                    //trace!("stream_rx::staging({} bytes)", amount);
                    continue;
                }
            }

            // Read some more data and put it in the buffer
            let data = rx.recv().await?;
            //trace!("stream_rx::recv_and_buffered({} bytes)", data.len());
            self.buffer.replace(Bytes::from(data));
        }

        // Success
        //trace!("stream_rx::read({} bytes)", index);
        Ok(())
    }

    async fn read_with_8bit_header(
        &mut self,
        rx: &mut StreamRxInner,
    ) -> Result<Vec<u8>, tokio::io::Error>
    {
        let len = self.read_u8(rx).await?;
        if len <= 0 {
            //trace!("stream_rx::8bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        //trace!("stream_rx::8bit-header(next_msg={} bytes)", len);
        let mut bytes = vec![0 as u8; len as usize];
        self.read_exact(rx, &mut bytes).await?;
        Ok(bytes)
    }
}

#[async_trait]
impl MessageProtocolApi
for MessageProtocol
{
    async fn write_with_fixed_16bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> { 
        self.write_with_16bit_header(tx, buf, delay_flush).await
    }

    async fn write_with_fixed_32bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {   
        self.write_with_32bit_header(tx, buf, delay_flush).await
    }

    async fn send(
        &mut self,
        tx: &mut StreamTxInner,
        wire_encryption: &Option<EncryptKey>,
        data: &[u8],
    ) -> Result<u64, tokio::io::Error> {
        let mut total_sent = 0u64;
        match wire_encryption {
            Some(key) => {
                let enc = key.encrypt(data);
                total_sent += self.write_with_8bit_header(tx, &enc.iv.bytes, true).await?;
                total_sent += self.write_with_32bit_header(tx, &enc.data, false).await?;
            }
            None => {
                total_sent += self.write_with_32bit_header(tx, data, false).await?;
            }
        };
        Ok(total_sent)
    }

    async fn read_with_fixed_16bit_header(
        &mut self,
        rx: &mut StreamRxInner,
    ) -> Result<Vec<u8>, tokio::io::Error>
    {
        let len = self.read_u16(rx).await?;
        if len <= 0 {
            //trace!("stream_rx::16bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        //trace!("stream_rx::16bit-header(next_msg={} bytes)", len);
        let mut bytes = vec![0 as u8; len as usize];
        self.read_exact(rx, &mut bytes).await?;
        Ok(bytes)
    }

    async fn read_with_fixed_32bit_header(
        &mut self,
        rx: &mut StreamRxInner,
    ) -> Result<Vec<u8>, tokio::io::Error>
    {
        let len = self.read_u32(rx).await?;
        if len <= 0 {
            //trace!("stream_rx::32bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        //trace!("stream_rx::32bit-header(next_msg={} bytes)", len);
        let mut bytes = vec![0 as u8; len as usize];
        self.read_exact(rx, &mut bytes).await?;
        Ok(bytes)
    }

    async fn read_buf_with_header(
        &mut self,
        rx: &mut StreamRxInner,
        wire_encryption: &Option<EncryptKey>,
        total_read: &mut u64
    ) -> std::io::Result<Vec<u8>>
    {
        match wire_encryption {
            Some(key) => {
                // Read the initialization vector
                let iv_bytes = self.read_with_8bit_header(rx).await?;
                *total_read += 1u64;
                match iv_bytes.len() {
                    0 => Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "iv_bytes-len is zero")),
                    l => {
                        *total_read += l as u64;
                        let iv = InitializationVector::from(iv_bytes);

                        // Read the cipher text and decrypt it
                        let cipher_bytes = self.read_with_fixed_32bit_header(rx).await?;
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
                let buf = self.read_with_fixed_32bit_header(rx).await?;
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
}