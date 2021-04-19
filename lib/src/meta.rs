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
    Nobody,
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

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct MetaAuthorization
{
    pub read: ReadOption,
    pub write: WriteOption,
}

impl MetaAuthorization
{
    pub fn is_relevant(&self) -> bool {
        self.read != ReadOption::Inherit || self.write != WriteOption::Inherit
    }
}

impl std::fmt::Display
for MetaAuthorization
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match &self.read {
            ReadOption::Everyone => "everyone".to_string(),
            ReadOption::Inherit => "inherit".to_string(),
            ReadOption::Specific(a) => format!("specific-{}", a),
        };
        let w = match &self.write {
            WriteOption::Everyone => "everyone".to_string(),
            WriteOption::Nobody => "nobody".to_string(),
            WriteOption::Inherit => "inherit".to_string(),
            WriteOption::Specific(a) => format!("specific-{}", a),
            WriteOption::Group(a) => {
                let mut r = "group".to_string();
                for a in a {
                    r.push_str("-");
                    r.push_str(a.to_string().as_str());
                }
                r
            }
        };
        write!(f, "(r:{}, w:{})", r, w)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct MetaCollection
{
    pub parent_id: PrimaryKey,
    pub collection_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
pub struct MetaType
{
    pub type_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaDelayedUpload
{
    pub complete: bool,
    pub from: Hash,
    pub to: Hash,
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
    Type(MetaType),
    Reply(PrimaryKey),
    DelayedUpload(MetaDelayedUpload),
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
                CoreMetadata::DelayedUpload(_) => {},
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

    pub fn get_type_name(&self) -> Option<&MetaType> {
        self.core
            .iter()
            .filter_map(|m| {
                match m {
                    CoreMetadata::Type(t) => Some(t),
                    _ => None,
                }
            })
            .next()
    }

    pub fn is_of_type<T: ?Sized>(&self) -> bool {
        if let Some(m) = self.core
            .iter()
            .filter_map(|m| {
                match m {
                    CoreMetadata::Type(t) => Some(t),
                    _ => None,
                }
            })
            .next()
        {
            return m.type_name == std::any::type_name::<T>().to_string()
        }

        false
    }

    pub fn set_type_name<T: ?Sized>(&mut self) {
        let type_name = std::any::type_name::<T>().to_string();
        
        if self.core
            .iter_mut()
            .filter_map(|m| {
                match m {
                    CoreMetadata::Type(t) => {
                        t.type_name = type_name.clone();
                        Some(t)
                    },
                    _ => None,
                }
            })
            .next()
            .is_none()
        {
            self.core.push(CoreMetadata::Type(MetaType {
                type_name,
            }));
        }
    }

    pub fn is_reply_to_what(&self) -> Option<PrimaryKey> {
        self.core
            .iter()
            .filter_map(|m| {
                match m {
                    CoreMetadata::Reply(a) => Some(a.clone()),
                    _ => None,
                }
            })
            .next()
    }

    pub fn get_delayed_upload(&self) -> Option<MetaDelayedUpload> {
        self.core.iter().filter_map(
            |m| {
                match m
                {
                    CoreMetadata::DelayedUpload(k) => Some(k.clone()),
                     _ => None
                }
            }
        )
        .next()
    }

    pub fn include_in_history(&self) -> bool {
        if self.get_delayed_upload().is_some() {
            return false;
        }
        true
    }
}