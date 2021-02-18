extern crate uuid;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header<M> {
    pub key: String,
    pub castle_id: Uuid,
    pub inherit_read: bool,
    pub inherit_write: bool,
    pub allow_read: Vec<String>,
    pub allow_write: Vec<String>,
    pub implicit_authority: String,
    pub version: Uuid,
    pub previous_version: Uuid,
    pub digest: Digest,
    pub meta: M,
}

#[allow(dead_code)]
pub type DefaultHeader = Header<EmptyMeta>;