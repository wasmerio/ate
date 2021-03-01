use serde::{Serialize, Deserialize, de::DeserializeOwned};
use super::crypto::*;

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

pub trait OtherMetadata
where Self: Serialize + DeserializeOwned + std::fmt::Debug + Default + Clone + Sized
{
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoreMetadata
{
    None,
    Encrypted(EncryptKey),
    EncryptedWith(PrimaryKey),
    Tombstone,
    InitializationVector([u8; 16]),
    Authorization {
        allow_read: Vec<String>,
        allow_write: Vec<String>,
        implicit_authority: String,
    },
    Tree {
        parent: PrimaryKey,
        inherit_read: bool,
        inherit_write: bool,
    },
    Digest {
        seed: Vec<u8>,
        digest: Vec<u8>,
    },
    Signature {
        signature: Vec<u8>,
        public_key_hash: String,
    },
    Author(String),
}

impl Default for CoreMetadata {
    fn default() -> Self {
        CoreMetadata::None
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct EmptyMetadata { }
impl OtherMetadata for EmptyMetadata { }

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Metadata<M>
{
    pub core: Vec<CoreMetadata>,
    pub other: M,
}

#[allow(dead_code)]
pub type DefaultMetadata = Metadata<EmptyMetadata>;

#[derive(Debug, Clone)]
pub struct Header<M>
where M: OtherMetadata
{
    pub key: PrimaryKey,
    pub meta: Metadata<M>
    
}
#[derive(Debug, Clone)]
pub struct HeaderData
{
    pub key: PrimaryKey,
    pub meta: Bytes,
    pub pointer: LogFilePointer,
}