extern crate uuid;

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::hash::{Hash};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmptyMeta {
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Digest {
    pub seed: Vec<u8>,
    pub signature: Vec<u8>,
    pub digest: Vec<u8>,
    pub public_key_hash: String,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
pub struct Header
{
    pub key: String,
    pub castle_id: Uuid,
    pub inherit_read: bool,
    pub inherit_write: bool,
    pub allow_read: Vec<String>,
    pub allow_write: Vec<String>,
    pub implicit_authority: String,
    
    pub version: Uuid,
    pub previous_version: Uuid,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HeaderIndex
{
    pub key: String,
    pub version: Uuid,
}

impl Header
{
    pub fn index(&self) -> HeaderIndex
    {
        HeaderIndex {
            key: self.key.clone(),
            version: self.version
        }
    }
}