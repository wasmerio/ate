use serde::{Serialize, Deserialize};

#[allow(unused_imports)]
use fastrand::u64;
use tokio::fs::File;
use bytes::Bytes;
use std::{hash::{Hash}, mem::size_of};
use tokio::io::Result;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}};
use tokio::{io::{BufStream}};
use super::redo::LogFilePointer;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PrimaryKey
{
    key: u64,
}

impl Default for PrimaryKey
{
    fn default() -> PrimaryKey {
        PrimaryKey::generate()
    }
}

impl PrimaryKey {
    #[allow(dead_code)]
    pub fn generate() -> PrimaryKey {
        PrimaryKey {
            key: fastrand::u64(..),
        }
    }

    pub async fn read_from_stream(reader: &mut BufStream<File>) -> Result<Option<PrimaryKey>> {
        let mut buf = [0 as u8; std::mem::size_of::<PrimaryKey>()];

        let read = reader.read(&mut buf).await?;
        if read == 0 { return Ok(None); }
        if read != buf.len() {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the right number of bytes for PrimaryKey ({:?} vs {:?})", read, buf.len())));
        }

        Ok(
            Some(
                PrimaryKey {
                    key: u64::from_be_bytes(buf)
                }
            )
        )
    }

    pub fn new(key: u64) -> PrimaryKey {
        PrimaryKey {
            key: key
        }
    }

    pub async fn write(&self, writer: &mut BufStream<File>) -> Result<()> {
        writer.write(&self.key.to_be_bytes()).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn sizeof() -> u64 {
        size_of::<u64>() as u64
    }

    pub fn as_hex_string(&self) -> String {
        format!("{:X?}", self.key).to_string()
    }
}

#[derive(Debug, Clone)]
pub struct HeaderData
{
    pub key: PrimaryKey,
    pub meta: Bytes,
    pub pointer: LogFilePointer,
}