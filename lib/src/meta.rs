use fxhash::FxHashSet;
use serde::{Serialize, Deserialize};
use crate::signature::MetaSignWith;

use super::crypto::*;
use super::header::*;
use super::signature::MetaSignature;

/// Determines if the event record will be restricted so that
/// only a specific set of users can read the data. If it is
/// limited to a specific set of users they must all possess
/// the encryption key in their session when accessing these
/// data records of which the hash of the encryption key must
/// match this record.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadOption
{
    Inherit,
    Everyone,
    Noone,
    Specific(Hash)
}

impl Default
for ReadOption
{
    fn default() -> ReadOption {
        ReadOption::Inherit
    }
}

/// Determines who is allowed to attach events records to this part of the
/// chain-of-trust key. Only users who have the `PrivateKey` in their session
/// will be able to write these records to the chain. The hash of the `PublicKey`
/// side is stored in this enum.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WriteOption
{
    Inherit,
    Everyone,
    Noone,
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
            WriteOption::Inherit => self,
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
        WriteOption::Inherit
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MetaAuthorization
{
    pub read: ReadOption,
    pub write: WriteOption,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct MetaCollection
{
    pub parent_id: PrimaryKey,
    pub collection_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaParent
{
    pub vec: MetaCollection,
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
    PublicKey(PublicSignKey),
    EncryptedPrivateKey(EncryptedPrivateKey),
    Confidentiality(ReadOption),
    Collection(MetaCollection),
    Parent(MetaParent),
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
pub struct Metadata
{
    pub core: Vec<CoreMetadata>,
}

impl Metadata
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

    pub fn get_effective_authorization(&self, inherit_auth: Option<MetaAuthorization>) -> MetaAuthorization
    {
        let auth = match self.get_authorization() {
            Some(a) => a.clone(),
            None => MetaAuthorization::default(),
        };
        let (inherit_read, inherit_write) = match inherit_auth {
            Some(a) => (a.read, a.write),
            None => (ReadOption::Everyone, WriteOption::Everyone)
        };
        MetaAuthorization {
            read: match auth.read {
                ReadOption::Inherit => inherit_read,
                a => a,
            },
            write: match auth.write {
                WriteOption::Inherit => inherit_write,
                a => a,
            }
        }
    }

    pub fn get_parent(&self) -> Option<&MetaParent>
    {
        for core in &self.core {
            if let CoreMetadata::Parent(a) = core {
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

    pub fn get_timestamp(&self) -> Option<&MetaTimestamp> {
        self.core
            .iter()
            .filter_map(|m| {
                match m {
                    CoreMetadata::Timestamp(time) => Some(time),
                    _ => None,
                }
            })
            .next()
    }
}