use std::io;
use std::ops::DerefMut;
use derivative::*;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use async_trait::async_trait;
use ate_crypto::{InitializationVector, EncryptKey};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::MessageProtocolApi;
use super::StreamRx;
use super::StreamTx;

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
    Close = 4,
}
impl MessageOpCode {
    fn to_u8(self) -> u8 {
        self as u8
    }
}
const MAX_MESSAGE_OP_CODE: u8 = 8;
const EXCESS_OP_CODE_SPACE: u8 = u8::MAX - MAX_MESSAGE_OP_CODE;
const MESSAGE_MAX_IV_REUSE: u32 = 1000;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MessageProtocol
{
    iv_tx: Option<InitializationVector>,
    iv_rx: Option<InitializationVector>,
    iv_use_cnt: u32,
    flip_to_abort: bool,
    is_closed: bool,
    #[derivative(Debug = "ignore")]
    rx: Option<Box<dyn AsyncRead + Send + Sync + Unpin + 'static>>,
    #[derivative(Debug = "ignore")]
    tx: Option<Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>>,
}

impl MessageProtocol
{
    pub(super) fn new(rx: Option<Box<dyn AsyncRead + Send + Sync + Unpin + 'static>>, tx: Option<Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>>) -> Self {
        Self {
            iv_tx: None,
            iv_rx: None,
            iv_use_cnt: 0,
            flip_to_abort: false,
            is_closed: false,
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
    
    #[allow(unused_variables)]
    async fn write_with_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        let tx = self.tx_guard()?;
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
            tx.write_all(&concatenated[..]).await?;
            if delay_flush == false {
                tx.flush().await?;
            }
            Ok(concatenated.len() as u64)
        } else {
            let total_sent = op.len() as u64 + len as u64;
            tx.write_all(&op[..]).await?;
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

    pub fn check_abort(&mut self) -> std::io::Result<bool>
    {
        if self.is_closed {
            if self.flip_to_abort {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            self.flip_to_abort = true;
            Ok(true)
        } else {
            Ok(false)
        }
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
        if self.check_abort()? {
            return Ok(0);
        }
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
        let tx = self.tx_guard()?;
        tx.write_all(&header[..]).await?;
        tx.write_all(&buf[..]).await?;
        if delay_flush == false {
            tx.flush().await?;
        }
        Ok(total_sent)     
    }

    async fn write_with_fixed_32bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        if self.check_abort()? {
            return Ok(0);
        }
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
        let tx = self.tx_guard()?;
        tx.write_all(&header[..]).await?;
        tx.write_all(&buf[..]).await?;
        if delay_flush == false {
            tx.flush().await?;
        }
        Ok(total_sent)     
    }

    async fn send(
        &mut self,
        wire_encryption: &Option<EncryptKey>,
        data: &[u8],
    ) -> Result<u64, tokio::io::Error> {
        if self.check_abort()? {
            return Ok(0);
        }
        let mut total_sent = 0u64;
        match wire_encryption {
            Some(key) => {
                if self.iv_tx.is_none() || self.iv_use_cnt > MESSAGE_MAX_IV_REUSE {
                    let iv = InitializationVector::generate();
                    let op = MessageOpCode::NewIV as u8;
                    let op = op.to_be_bytes();
                    let concatenated = [&op, &iv.bytes[..]].concat();
                    self.tx_guard()?.write_all(&concatenated[..]).await?;
                    self.iv_tx.replace(iv);
                    self.iv_use_cnt = 0;
                } else {
                    self.iv_use_cnt += 1;
                }

                let iv = self.iv_tx.as_ref().unwrap();                
                let enc = key.encrypt_with_iv(iv, data);
                total_sent += self.write_with_header(&enc[..], false).await?;
            }
            None => {
                total_sent += self.write_with_header(data, false).await?;
            }
        };
        Ok(total_sent)
    }

    async fn read_with_fixed_16bit_header(
        &mut self,
    ) -> Result<Vec<u8>, tokio::io::Error>
    {
        if self.check_abort()? {
            return Ok(vec![]);
        }
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
        if self.check_abort()? {
            return Ok(vec![]);
        }
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
        // Enter a loop processing op codes and the data within it
        loop {
            if self.check_abort()? {
                return Ok(vec![]);
            }
            let op = self.read_u8().await?;
            *total_read += 1;
            let len = if op == MessageOpCode::Noop.to_u8() {
                //trace!("stream_rx::op(noop)");
                continue;
            } else if op == MessageOpCode::NewIV.to_u8() {
                //trace!("stream_rx::op(new-iv)");
                let mut iv = [0u8; 16];
                self.read_exact(&mut iv).await?;
                *total_read += 16;
                let iv: InitializationVector = (&iv[..]).into();
                self.iv_rx.replace(iv);
                continue;
            } else if op == MessageOpCode::Buf16bit.to_u8() {
                //trace!("stream_rx::op(buf-16bit)");
                let len = self.read_u16().await? as u32;
                *total_read += 2;
                len
            } else if op == MessageOpCode::Buf32bit.to_u8() {
                //trace!("stream_rx::op(buf-32bit)");
                let len = self.read_u32().await? as u32;
                *total_read += 4;
                len
            } else if op == MessageOpCode::Close.to_u8() {
                //trace!("stream_rx::op(close)");
                self.is_closed = true;
                continue;

            } else {
                //trace!("stream_rx::op(buf-packed)");
                (op as u32) - (MAX_MESSAGE_OP_CODE as u32)
            };

            // Now read the data
            //trace!("stream_rx::buf(len={})", len);
            let mut bytes = vec![0 as u8; len as usize];
            self.read_exact(&mut bytes).await?;
            *total_read += len as u64;

            // If its encrypted then we need to decrypt
            if let Some(key) = wire_encryption {
                // Get the initialization vector
                if self.iv_rx.is_none() {
                    return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "iv is missing"));
                }
                let iv = self.iv_rx.as_ref().unwrap();

                // Decrypt the bytes
                //trace!("stream_rx::decrypt(len={})", len);
                bytes = key.decrypt(iv, &bytes[..]);
            }

            // Return the result
            //trace!("stream_rx::ret(len={})", len);
            return Ok(bytes);
        }
    }

    async fn send_close(
        &mut self,
    ) -> std::io::Result<()> {
        let tx = self.tx_guard()?;
        let op = MessageOpCode::Close as u8;
        let op = op.to_be_bytes();
        tx.write_all(&op[..]).await?;
        tx.flush().await?;
        self.is_closed = true;
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
