use serde::{Serialize, Deserialize};
use super::crypto::*;

#[allow(unused_imports)]
use fastrand::u64;
use tokio::fs::File;
use bytes::Bytes;
use std::{hash::{Hash}, mem::size_of};
use tokio::io::Result;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}};
use tokio::{io::{BufStream}};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct PrimaryKey
{
    pub key: u64,
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

    pub async fn read(reader: &mut BufStream<File>) -> Result<PrimaryKey> {
        Ok(
            PrimaryKey {
                key: reader.read_u64().await?
            }
        )
    }

    pub async fn write(&self, writer: &mut BufStream<File>) -> Result<()> {
        writer.write_u64(self.key).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn sizeof() -> u64 {
        size_of::<u64>() as u64
    }
}
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct MetaCastle
{
    pub key: EncryptKey,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct MetaConfidentiality
{
    pub castle_id: PrimaryKey,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct MetaAuthorization
{
    pub allow_read: Vec<String>,
    pub allow_write: Vec<String>,
    pub implicit_authority: String,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct MetaTree
{
    pub parent: PrimaryKey,
    pub inherit_read: bool,
    pub inherit_write: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct MetaDigest {
    pub seed: Vec<u8>,
    pub digest: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct MetaSignature {
    pub signature: Vec<u8>,
    pub public_key_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct DefaultMeta {
    pub tree: Option<MetaTree>,
    pub castle: Option<MetaCastle>,
    pub confidentiality: Option<MetaConfidentiality>,
    pub auth: Option<MetaAuthorization>,
    pub digest: Option<MetaDigest>,
    pub signature: Option<MetaSignature>,
}

#[derive(Debug, Clone)]
pub struct Header<M> {
    pub key: PrimaryKey,
    pub meta: M,
}
#[derive(Debug, Clone)]
pub struct HeaderData
{
    pub key: PrimaryKey,
    pub meta: Bytes
}