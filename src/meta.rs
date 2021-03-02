use serde::{Serialize, Deserialize, de::DeserializeOwned};
use super::crypto::*;
use super::header::*;

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