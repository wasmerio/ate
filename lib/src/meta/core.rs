use serde::{Serialize, Deserialize};
use crate::signature::MetaSignWith;

use crate::crypto::*;
use crate::error::CryptoError;
use crate::header::*;
use crate::signature::MetaSignature;
use crate::time::*;

use super::*;

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
    Confidentiality(MetaConfidentiality),
    Collection(MetaCollection),
    Parent(MetaParent),
    Timestamp(ChainTimestamp),
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

impl std::fmt::Display
for CoreMetadata
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreMetadata::None => write!(f, "none"),
            CoreMetadata::Data(a) => write!(f, "data-{}", a),
            CoreMetadata::Tombstone(a) => write!(f, "tombstone-{}", a),
            CoreMetadata::Authorization(a) => write!(f, "auth({})", a),
            CoreMetadata::InitializationVector(a) => write!(f, "iv({})", a),
            CoreMetadata::PublicKey(a) => write!(f, "public_key({})", a.hash()),
            CoreMetadata::EncryptedPrivateKey(a) => write!(f, "encrypt_private_key({})", a.as_public_key().hash()),
            CoreMetadata::Confidentiality(a) => write!(f, "confidentiality-{}", a),
            CoreMetadata::Collection(a) => write!(f, "collection-{}", a ),
            CoreMetadata::Parent(a) => write!(f, "parent-{}", a),
            CoreMetadata::Timestamp(a) => write!(f, "timestamp-{}", a),
            CoreMetadata::Signature(a) => write!(f, "signature-{}", a),
            CoreMetadata::SignWith(a) => write!(f, "sign_with({})", a),
            CoreMetadata::Author(a) => write!(f, "author-{}", a),
            CoreMetadata::Type(a) => write!(f, "type-{}", a),
            CoreMetadata::Reply(a) => write!(f, "reply-{}", a),
            CoreMetadata::DelayedUpload(a) => write!(f, "delayed_upload-{}", a),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Metadata
{
    pub core: Vec<CoreMetadata>,
}

impl std::fmt::Display
for Metadata
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        write!(f, "meta[")?;
        for core in self.core.iter() {
            if first {
                first = false;
            } else {
                write!(f, ",")?;
            }
            core.fmt(f)?;
        }
        write!(f, "]")
    }
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

    pub fn get_confidentiality(&self) -> Option<&MetaConfidentiality>
    {
        for core in &self.core {
            if let CoreMetadata::Confidentiality(a) = core {
                return Some(a);
            }
        }
        None
    }

    pub fn generate_iv(&mut self) -> InitializationVector {
        let mut core = self.core.clone()
            .into_iter()
            .filter(|m|  match m {
                CoreMetadata::InitializationVector(_) => false,
                _ => true,
            })
            .collect::<Vec<_>>();
        
        let iv = InitializationVector::generate();
        core.push(CoreMetadata::InitializationVector(iv.clone()));
        self.core = core;
        return iv;
    }

    pub fn get_iv(&self) -> Result<&InitializationVector, CryptoError> {
        for m in self.core.iter() {
            match m {
                CoreMetadata::InitializationVector(iv) => return Result::Ok(iv),
                _ => { }
            }
        }
        Result::Err(CryptoError::NoIvPresent)
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

    pub fn get_timestamp(&self) -> Option<&ChainTimestamp> {
        for core in &self.core {
            if let CoreMetadata::Timestamp(a) = core {
                return Some(a);
            }
        }        
        None
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