use fxhash::FxHashSet;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::signature::MetaSignWith;

use super::crypto::*;
use super::header::*;
use super::signature::MetaSignature;

pub trait OtherMetadata
where Self: Serialize + DeserializeOwned + std::fmt::Debug + Default + Clone + Sized
{
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReadOption
{
    Unspecified,
    Everyone,
    Specific(Hash)
}

impl Default
for ReadOption
{
    fn default() -> ReadOption {
        ReadOption::Unspecified
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WriteOption
{
    Unspecified,
    Everyone,
    Specific(Hash),
    Group(Vec<Hash>)
}

impl WriteOption
{
    pub fn vals(&self) -> FxHashSet<Hash> {
        let mut ret = FxHashSet::default();
        match self {
            WriteOption::Specific(a) => { ret.insert(a.clone()); }
            WriteOption::Group(hashes) => {
                for a in hashes {
                    ret.insert(a.clone());
                }
            },
            _ => {}
        }
        return ret;
    }

    pub fn or(self, other: &WriteOption) -> WriteOption {
        match other {
            WriteOption::Unspecified => self,
            WriteOption::Group(keys) => {
                let mut vals = self.vals();
                for a in keys {
                    vals.insert(a.clone());
                }
                WriteOption::Group(vals.iter().map(|k| k.clone()).collect::<Vec<_>>())
            },
            WriteOption::Specific(hash) => {
                let mut vals = self.vals();
                vals.insert(hash.clone());
                WriteOption::Group(vals.iter().map(|k| k.clone()).collect::<Vec<_>>())
            },
            a => a.clone(),
        }
    }
}

impl Default
for WriteOption
{
    fn default() -> WriteOption {
        WriteOption::Unspecified
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MetaAuthorization
{
    pub allow_read: ReadOption,
    pub allow_write: WriteOption,
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
    Confidentiality(ReadOption),
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

    pub fn get_confidentiality(&self) -> Option<&ReadOption>
    {
        for core in &self.core {
            if let CoreMetadata::Confidentiality(a) = core {
                return Some(a);
            }
        }
        None
    }

    pub fn needs_signature(&self) -> bool
    {
        for core in &self.core {
            match core {
                CoreMetadata::PublicKey(_) => {},
                CoreMetadata::Signature(_) => {},
                CoreMetadata::EncryptedPrivateKey(_) => {},
                CoreMetadata::Confidentiality(_) => {},                
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