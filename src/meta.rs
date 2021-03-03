use serde::{Serialize, Deserialize, de::DeserializeOwned};
use super::crypto::*;
use super::header::*;

pub trait OtherMetadata
where Self: Serialize + DeserializeOwned + std::fmt::Debug + Default + Clone + Sized
{
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaAuthorization
{
    allow_read: Vec<Hash>,
    allow_write: Vec<Hash>,
    implicit_authority: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaTree
{
    pub parent: PrimaryKey,
    pub inherit_read: bool,
    pub inherit_write: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaDigest
{
    pub seed: Vec<u8>,
    pub digest: Hash,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaPublicKey
{
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaSignature
{
    pub digests: Vec<Hash>,
    pub signature: Vec<u8>,
    pub public_key_hash: Hash,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaInitializationVector
{
    pub iv: [u8; 16],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoreMetadata
{
    None,
    Data(PrimaryKey),
    Encrypted(EncryptKey),
    EncryptedWith(PrimaryKey),
    Tombstone(PrimaryKey),
    InitializationVector(MetaInitializationVector),
    Authorization(MetaAuthorization),
    PublicKey(MetaPublicKey),
    Tree(MetaTree),
    Digest(MetaDigest),
    Signature(MetaSignature),
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