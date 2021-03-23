use serde::{Serialize, Deserialize};
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

use async_trait::async_trait;
use super::error::*;
use tokio::io::ErrorKind;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum SerializationFormat
{
    Json = 1,
    MessagePack = 2,
    Bincode = 3,
}

impl std::str::FromStr
for SerializationFormat
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(SerializationFormat::Json),
            "messagepack" => Ok(SerializationFormat::MessagePack),
            "mpack" => Ok(SerializationFormat::MessagePack),
            "bincode" => Ok(SerializationFormat::Bincode),
            "bc" => Ok(SerializationFormat::Bincode),
            _ => Err("no match"),
        }
    }
}

impl SerializationFormat
{
    pub fn serialize<T>(&self, val: &T) -> Result<Vec<u8>, SerializationError>
    where T: Serialize + ?Sized
    {
        match self {
            SerializationFormat::Json => {
                Ok(serde_json::to_vec(val)?)
            },
            SerializationFormat::MessagePack => {
                Ok(rmp_serde::to_vec(val)?)
            },
            SerializationFormat::Bincode => {
                Ok(bincode::serialize(val)?)
            }
        }
    }

    pub fn deserialize<'a, T>(&self, val: &'a [u8]) -> Result<T, SerializationError>
    where T: serde::de::Deserialize<'a>
    {
        match self {
            SerializationFormat::Json => {
                Ok(serde_json::from_slice(val)?)
            },
            SerializationFormat::MessagePack => {
                Ok(rmp_serde::from_read_ref(val)?)
            },
            SerializationFormat::Bincode => {
                Ok(bincode::deserialize(val)?)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageFormat
{
    pub meta: SerializationFormat,
    pub data: SerializationFormat,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum BlobSize
{
    U8 = 1,
    U16 = 2,
    U32 = 3,
    U64 = 4,
}

static LOG_MAGIC: &'static [u8; 3] = b"Ate";

#[async_trait]
pub trait LogApi
{
    fn offset(&self) -> u64;
    
    async fn read_u8(&mut self) -> Result<u8, tokio::io::Error>;
    async fn read_u16(&mut self) -> Result<u16, tokio::io::Error>;
    async fn read_u32(&mut self) -> Result<u32, tokio::io::Error>;
    async fn read_u64(&mut self) -> Result<u64, tokio::io::Error>;
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), tokio::io::Error>;

    async fn write_u8(&mut self, val: u8) -> Result<(), tokio::io::Error>;
    async fn write_u16(&mut self, val: u16) -> Result<(), tokio::io::Error>;
    async fn write_u32(&mut self, val: u32) -> Result<(), tokio::io::Error>;
    async fn write_u64(&mut self, val: u64) -> Result<(), tokio::io::Error>;
    async fn write_exact(&mut self, buf: &[u8]) -> Result<(), tokio::io::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogHeader
{
    pub offset: u64,
    pub format: MessageFormat,
}

#[derive(Debug, Clone)]
pub struct LogEntry
{
    pub header: LogHeader,
    pub meta: Vec<u8>,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum LogVersion
{
    V1 = b'!',
    V2 = b'1',
}

impl LogVersion
{
    async fn read_byte(api: &mut impl LogApi) -> std::result::Result<Option<u8>, SerializationError>
    {
        match api.read_u8().await
        {
            Ok(a) => Ok(Some(a)),
            Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof { return Ok(None); }
                Err(SerializationError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the event magic number at 0x{:x}", api.offset()))))
            },
        }
    }

    async fn read_version(api: &mut impl LogApi) -> std::result::Result<Option<LogVersion>, SerializationError>
    {
        let mut n = 0;
        while let Some(cur) = LogVersion::read_byte(api).await? {
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

                match LogVersion::try_from(cur) {
                    Ok(a) => { return Ok(Some(a)); },
                    _ => { 
                        n = 0;
                        continue
                    }
                }            
            }
        }

        return Ok(None);
    }

    async fn read_blob_size(&self, api: &mut impl LogApi) -> Result<usize, SerializationError> {
        match self {
            LogVersion::V1 => Ok(api.read_u32().await? as usize),
            LogVersion::V2 => {
                match BlobSize::try_from(api.read_u8().await?) {
                    Ok(BlobSize::U8) => Ok(api.read_u8().await? as usize),
                    Ok(BlobSize::U16) => Ok(api.read_u16().await? as usize),
                    Ok(BlobSize::U32) => Ok(api.read_u32().await? as usize),
                    Ok(BlobSize::U64) => Ok(api.read_u64().await? as usize),
                    Err(err) => {
                        Err(SerializationError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read data at 0x{:x} - {}", api.offset(), err))))
                    }
                }
            }
        }
    }

    async fn write_blob_size(&self, api: &mut impl LogApi, val: usize) -> Result<(), SerializationError> {
        match self {
            LogVersion::V1 => Ok(api.write_u32(val as u32).await?),
            LogVersion::V2 => {
                let blob_size = match val {
                    _ if val < u8::MAX as usize => BlobSize::U8,
                    _ if val < u16::MAX as usize => BlobSize::U16,
                    _ if val < u32::MAX as usize => BlobSize::U32,
                    _ if val < u64::MAX as usize => BlobSize::U64,
                    _ => BlobSize::U32,
                };
                api.write_u8(blob_size.into()).await?;
                match blob_size {
                    BlobSize::U8 => Ok(api.write_u8(val as u8).await?),
                    BlobSize::U16 => Ok(api.write_u16(val as u16).await?),
                    BlobSize::U32 => Ok(api.write_u32(val as u32).await?),
                    BlobSize::U64 => Ok(api.write_u64(val as u64).await?),
                }
            }
        }
    }

    async fn read_format(&self, api: &mut impl LogApi, default: SerializationFormat) -> Result<SerializationFormat, SerializationError> {
        match self {
            LogVersion::V1 => Ok(default),
            LogVersion::V2 => Ok(match SerializationFormat::try_from(api.read_u8().await?) {
                Ok(a) => a,
                Err(_) => {
                    return Err(SerializationError::InvalidSerializationFormat);
                }
            })
        }
    }

    async fn write_format(&self, api: &mut impl LogApi, format: SerializationFormat) -> Result<(), SerializationError> {
        match self {
            LogVersion::V2 => {
                match api.write_u8(format.into()).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err(SerializationError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to write data at 0x{:x} - {}", api.offset(), err))))
                }
            },
            _ => Ok(()),
        }
    }

    pub async fn read(api: &mut impl LogApi, mut format: MessageFormat) -> Result<Option<LogEntry>, SerializationError> {
        let offset = api.offset();

        let version = match Self::read_version(api).await? {
            Some(a) => a,
            None => { return Ok(None); }
        };
        
        format.meta = version.read_format(api, format.meta).await?;
        let meta_size = version.read_blob_size(api).await?;
        let mut meta = vec![0 as u8; meta_size];
        api.read_exact(&mut meta[..]).await?;

        format.data = version.read_format(api, format.data).await?;
        let data_size = version.read_blob_size(api).await?;
        let data = if data_size > 0 {
            let mut data = vec![0 as u8; data_size];
            api.read_exact(&mut data[..]).await?;
            Some(data)
        } else { None };

        Ok(Some(LogEntry {
            header: LogHeader {
                offset,
                format
            },
            meta,
            data
        }))
    }

    pub async fn write(&self, api: &mut impl LogApi, meta: &[u8], data: Option<&[u8]>, format: MessageFormat) -> Result<LogHeader, SerializationError> {
        let offset = api.offset();
        
        api.write_exact(&LOG_MAGIC[..]).await?;
        api.write_u8((*self).into()).await?;

        self.write_format(api, format.meta).await?;
        self.write_blob_size(api, meta.len()).await?;
        api.write_exact(&meta[..]).await?;

        self.write_format(api, format.data).await?;
        match data {
            Some(data) => {
                self.write_blob_size(api, data.len()).await?;
                api.write_exact(&data[..]).await?;
            },
            None => {
                self.write_blob_size(api, 0).await?;
            }
        };

        Ok(LogHeader {
            offset,
            format,
        })
    }
}