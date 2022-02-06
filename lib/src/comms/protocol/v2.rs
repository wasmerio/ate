#![allow(unused_imports)]
use async_trait::async_trait;
use bytes::Bytes;
use crate::{comms::stream::StreamTxInner, comms::stream::StreamRxInner, crypto::{InitializationVector, EncryptKey}};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::api::MessageProtocolApi;

/// Opcodes are used to build and send the messages over the websocket
/// they must not exceed 16! as numbers above 16 are used for buffers
/// that are smaller than 239 bytes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageOpCode {
    #[allow(dead_code)]
    Noop = 0,
    NewIV = 1,
    Buf16bit = 2,
    Buf32bit = 3,
}
impl MessageOpCode {
    fn to_u8(self) -> u8 {
        self as u8
    }
}
const MAX_MESSAGE_OP_CODE: u8 = 4;
const EXCESS_OP_CODE_SPACE: u8 = u8::MAX - MAX_MESSAGE_OP_CODE;
const MESSAGE_MAX_IV_REUSE: u32 = 1000;


#[derive(Debug, Default, Clone)]
pub struct MessageProtocol {
    iv_tx: Option<InitializationVector>,
    iv_rx: Option<InitializationVector>,
    iv_use_cnt: u32,
    buffer: Option<Bytes>,
}

impl MessageProtocol
{
    #[allow(unused_variables)]
    async fn write_with_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        let len = buf.len();
        let op = if len < EXCESS_OP_CODE_SPACE as usize {
            let len = len as u8;
            let op = MAX_MESSAGE_OP_CODE + len;
            vec![ op ]
        } else if len < u16::MAX as usize {
            let op = MessageOpCode::Buf16bit as u8;
            let len = len as u16;
            let len = len.to_be_bytes();
            vec![op, len[0], len[1]]
        } else if len < u32::MAX as usize {
            let op = MessageOpCode::Buf32bit as u8;
            let len = len as u32;
            let len = len.to_be_bytes();
            vec![op, len[0], len[1], len[2], len[3]]
        } else {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                format!(
                    "Data is too big to write (len={}, max={})",
                    buf.len(),
                    u32::MAX
                ),
            ));
        };

        if len <= 62 {
            let concatenated = [&op[..], &buf[..]].concat();
            tx.write_all(&concatenated[..], delay_flush).await?;
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = op.len() as u64 + len as u64;
            tx.write_all(&op[..], true).await?;
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
                    trace!("stream_rx::staging({} bytes)", amount);
                    continue;
                }
            }

            // Read some more data and put it in the buffer
            let data = rx.recv().await?;
            trace!("stream_rx::recv_and_buffered({} bytes)", data.len());
            self.buffer.replace(Bytes::from(data));
        }

        // Success
        trace!("stream_rx::read({} bytes)", index);
        Ok(())
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
        let len = buf.len();
        let header = if len < u16::MAX as usize {
            let len = len as u16;
            len.to_be_bytes()
        } else {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                format!(
                    "Data is too big to write (len={}, max={})",
                    buf.len(),
                    u16::MAX
                ),
            ));
        };

        let total_sent = header.len() as u64 + buf.len() as u64;
        tx.write_all(&header[..], true).await?;
        tx.write_all(&buf[..], delay_flush).await?;
        Ok(total_sent)     
    }

    async fn write_with_fixed_32bit_header(
        &mut self,
        tx: &mut StreamTxInner,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        let len = buf.len();
        let header = if len < u32::MAX as usize {
            let len = len as u32;
            len.to_be_bytes()
        } else {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                format!(
                    "Data is too big to write (len={}, max={})",
                    buf.len(),
                    u32::MAX
                ),
            ));
        };

        let total_sent = header.len() as u64 + buf.len() as u64;
        tx.write_all(&header[..], true).await?;
        tx.write_all(&buf[..], delay_flush).await?;
        Ok(total_sent)     
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
                if self.iv_tx.is_none() || self.iv_use_cnt > MESSAGE_MAX_IV_REUSE {
                    let iv = InitializationVector::generate();
                    let op = MessageOpCode::NewIV as u8;
                    let op = op.to_be_bytes();
                    let concatenated = [&op, &iv.bytes[..]].concat();
                    tx.write_all(&concatenated[..], true).await?;
                    self.iv_tx.replace(iv);
                    self.iv_use_cnt = 0;
                } else {
                    self.iv_use_cnt += 1;
                }

                let iv = self.iv_tx.as_ref().unwrap();                
                let enc = key.encrypt_with_iv(iv, data);
                total_sent += self.write_with_header(tx, &enc[..], false).await?;
            }
            None => {
                total_sent += self.write_with_header(tx, data, false).await?;
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
            trace!("stream_rx::16bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        trace!("stream_rx::16bit-header(next_msg={} bytes)", len);
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
            trace!("stream_rx::32bit-header(no bytes!!)");
            return Ok(vec![]);
        }
        trace!("stream_rx::32bit-header(next_msg={} bytes)", len);
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
        // Enter a loop processing op codes and the data within it
        loop {
            let op = self.read_u8(rx).await?;
            *total_read += 1;
            let len = if op == MessageOpCode::Noop.to_u8() {
                trace!("stream_rx::op(noop)");
                continue;
            } else if op == MessageOpCode::NewIV.to_u8() {
                trace!("stream_rx::op(new-iv)");
                let mut iv = [0u8; 16];
                self.read_exact(rx, &mut iv).await?;
                *total_read += 16;
                let iv: InitializationVector = (&iv[..]).into();
                self.iv_rx.replace(iv);
                continue;
            } else if op == MessageOpCode::Buf16bit.to_u8() {
                trace!("stream_rx::op(buf-16bit)");
                let len = self.read_u16(rx).await? as u32;
                *total_read += 2;
                len
            } else if op == MessageOpCode::Buf32bit.to_u8() {
                trace!("stream_rx::op(buf-32bit)");
                let len = self.read_u32(rx).await? as u32;
                *total_read += 4;
                len
            } else {
                trace!("stream_rx::op(buf-packed)");
                (op as u32) - (MAX_MESSAGE_OP_CODE as u32)
            };

            // Now read the data
            trace!("stream_rx::buf(len={})", len);
            let mut bytes = vec![0 as u8; len as usize];
            self.read_exact(rx, &mut bytes).await?;
            *total_read += len as u64;

            // If its encrypted then we need to decrypt
            if let Some(key) = wire_encryption {
                // Get the initialization vector
                if self.iv_rx.is_none() {
                    return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "iv is missing"));
                }
                let iv = self.iv_rx.as_ref().unwrap();

                // Decrypt the bytes
                trace!("stream_rx::decrypt(len={})", len);
                bytes = key.decrypt(iv, &bytes[..]);
            }

            // Return the result
            trace!("stream_rx::ret(len={})", len);
            return Ok(bytes);
        }
    }
}