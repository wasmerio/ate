use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::signature::MetaSignWith;

use super::crypto::*;
use super::header::*;
use super::signature::MetaSignature;

pub trait OtherMetadata
where Self: Serialize + DeserializeOwned + std::fmt::Debug + Default + Clone + Sized
{
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MetaAuthorization
{
    pub allow_read: Vec<Hash>,
    pub allow_write: Vec<Hash>,
    pub implicit_authority: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct MetaCollection
{
    pub parent_id: PrimaryKey,
    pub collection_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaTree
{
    pub vec: MetaCollection,
    pub inherit_read: bool,
    pub inherit_write: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaTimestamp
{
    pub time_since_epoch_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoreMetadata
{
    None,
    Data(PrimaryKey),
    Tombstone(PrimaryKey),
    Authorization(MetaAuthorization),
    InitializationVector(InitializationVector),
    PublicKey(PublicKey),
    EncryptedPrivateKey(EncryptedPrivateKey),
    EncryptedEncryptionKey(EncryptKey),
    Collection(MetaCollection),
    Tree(MetaTree),
    Timestamp(MetaTimestamp),
    Signature(MetaSignature),
    SignWith(MetaSignWith),
    Author(String),
}

impl Default for CoreMetadata {
    fn default() -> Self {
        CoreMetadata::None
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NoAdditionalMetadata { }
impl OtherMetadata for NoAdditionalMetadata { }

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct MetadataExt<M>
{
    pub core: Vec<CoreMetadata>,
    pub other: M,
}

#[allow(dead_code)]
pub type DefaultMetadata = MetadataExt<NoAdditionalMetadata>;

impl<M> MetadataExt<M>
{
    pub fn get_authorization(&self) -> Option<&MetaAuthorization>
    {
        for core in &self.core {
            match core {
                CoreMetadata::Authorization(a) => {
                    return Some(a);
                },
                _ => {}
            }
        }
        
        None
    }

    pub fn get_tree(&self) -> Option<&MetaTree>
    {
        for core in &self.core {
            if let CoreMetadata::Tree(a) = core {
                return Some(a);
            }
        }
        
        None
    }

    pub fn get_collections(&self) -> Vec<MetaCollection>
    {
        let mut ret = Vec::new();
        for core in &self.core {
            if let CoreMetadata::Collection(a) = core {
                ret.push(a.clone());
            }
        }        
        ret
    }

    pub fn needs_signature(&self) -> bool
    {
        for core in &self.core {
            match core {
                CoreMetadata::PublicKey(_) => {},
                CoreMetadata::Signature(_) => {},
                CoreMetadata::EncryptedPrivateKey(_) => {},
                CoreMetadata::EncryptedEncryptionKey(_) => {},                
                _ => { return true; }
            }
        }

        false
    }

    pub fn get_sign_with(&self) -> Option<&MetaSignWith>
    {
        for core in &self.core {
            if let CoreMetadata::SignWith(a) = core {
                return Some(a);
            }
        }
        
        None
    }
}