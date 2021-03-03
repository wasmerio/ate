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
pub enum CoreMetadata
{
    None,
    Data(PrimaryKey),
    Encrypted(EncryptKey),
    EncryptedWith(PrimaryKey),
    Tombstone(PrimaryKey),
    InitializationVector([u8; 16]),
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

impl<M> Metadata<M>
where M: Default
{
    pub fn for_data(key: PrimaryKey) -> Metadata<M> {
        let mut ret = Metadata::default();
        ret.core.push(CoreMetadata::Data(key));
        return ret;
    }
    
    pub fn get_data_key(&self) -> Option<PrimaryKey> {
        self.core.iter().filter_map(
            |m| {
                match m
                {
                    CoreMetadata::Data(k) => Some(k.clone()),
                     _ => None
                }
            }
        )
        .next()
    }

    #[allow(dead_code)]
    pub fn set_data_key(&mut self, key: PrimaryKey) {
        for core in self.core.iter_mut() {
            match core {
                CoreMetadata::Data(k) => {
                    if *k == key { return; }
                    *k = key;
                    return;
                },
                _ => {}
            }
        }
        self.core.push(CoreMetadata::Data(key));
    }
}

#[allow(dead_code)]
pub type DefaultMetadata = Metadata<EmptyMetadata>;