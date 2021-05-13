use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use tokio::io::ErrorKind;

use crate::spec::LogApi;
use crate::redo::appender::LogAppender;

static LOG_MAGIC: &'static [u8; 3] = b"RED";

#[derive(Debug, Clone, Copy, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum RedoMagic
{
    #[deprecated(
        since = "0.6.1",
        note = "This redo magic header is deprecated and will be removed in a future release."
    )]
    V1 = b'O',
    V2 = b'1',
}

#[derive(Debug, Clone)]
pub struct RedoHeader
{
    magic: RedoMagic,
    inner: Vec<u8>,
}

async fn read_byte(api: &mut impl LogApi) -> std::result::Result<Option<u8>, tokio::io::Error>
{
    match api.read_u8().await
    {
        Ok(a) => Ok(Some(a)),
        Err(err) => {
            if err.kind() == ErrorKind::UnexpectedEof { return Ok(None); }
            Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the event magic number at 0x{:x}", api.offset())))
        },
    }
}

impl RedoHeader
{
    pub fn new(magic: RedoMagic) -> RedoHeader
    {
        RedoHeader {
            magic,
            inner: Vec::new(),
        }
    }

    pub(crate) async fn load(appender: &mut LogAppender, default_header_bytes: &[u8]) -> Result<Vec<u8>, tokio::io::Error> {
        Ok(
            match RedoHeader::read(appender).await? {
                Some(a) => {
                    Vec::from(a.inner().clone())
                },
                None => {
                    let mut magic = RedoHeader::new(RedoMagic::V2);
                    magic.set_inner(default_header_bytes);
                    let _ = magic.write(appender).await?;
                    appender.sync().await?;
                    Vec::from(default_header_bytes)
                }
            }
        )
    }

    pub async fn read(api: &mut impl LogApi) -> Result<Option<RedoHeader>, tokio::io::Error>
    {
        let mut n = 0;
        while let Some(cur) = read_byte(api).await? {
            loop {
                if n < LOG_MAGIC.len() {
                    if cur == LOG_MAGIC[n] {
                        n = n + 1;
                        break;
                    }
                    if n > 0 { 
                        n = 0;
                        continue;
                    }
                    break;
                }

                match RedoMagic::try_from(cur) {
                    Ok(a) => {
                        let inner = match a {
                            #[allow(deprecated)]
                            RedoMagic::V1 => Vec::new(),
                            RedoMagic::V2 => {
                                let inner_size = api.read_u32().await?;
                                let mut inner = vec![0 as u8; inner_size as usize];
                                api.read_exact(&mut inner[..]).await?;
                                inner
                            }
                        };

                        return Ok(Some(
                            RedoHeader {
                                magic: a,
                                inner,
                            }
                        ));
                    },
                    _ => { 
                        n = 0;
                        continue
                    }
                }            
            }
        }

        return Ok(None);
    }

    pub async fn write(self, api: &mut impl LogApi) -> Result<(), tokio::io::Error> {
        api.write_exact(&LOG_MAGIC[..]).await?;
        api.write_u8(self.magic.into()).await?;

        match self.magic {
            RedoMagic::V2 => {
                api.write_u32(self.inner.len() as u32).await?;
                api.write_exact(&self.inner[..]).await?;
            }
            _ => { }
        }

        Ok(())
    }

    pub fn inner(&self) -> &[u8] {
        &self.inner[..]
    }

    pub fn set_inner(&mut self, val: &[u8]) {
        self.inner = Vec::from(val);
    }
}