#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use super::crypto::*;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SessionProperty
{
    None,
    ReadKey(EncryptKey),
    WriteKey(PrivateKey),
    Identity(String),
}

impl Default for SessionProperty {
    fn default() -> Self {
        SessionProperty::None
    }
}

/// Sessions hold facts about the user that give them certains
/// rights and abilities to view data within the chain-of-trust.
///
/// For instance - to see encrypted data for specific users they
/// must insert their `EncryptKey` into this session before
/// accessing the chain via a `Dio`.
///
/// Another example is the ability to write data. For certain
/// records within the tree if they have been protected with
/// write protection then you must insert your `PrivateKey`
/// into the session before you attempt to insert or modify these
/// records.
///
/// Sessions are never cached and only exist in memory for the
/// duration that you use them for security reasons.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Session
where Self: Send + Sync
{
    pub properties: Vec<SessionProperty>,
}

impl Session
{
    #[allow(dead_code)]
    pub fn add_read_key(&mut self, key: &EncryptKey) {
        self.properties.push(SessionProperty::ReadKey(key.clone()));
    }

    #[allow(dead_code)]
    pub fn add_write_key(&mut self, key: &PrivateKey) {
        self.properties.push(SessionProperty::WriteKey(key.clone()));
    }

    #[allow(dead_code)]
    pub fn add_identity(&mut self, identity: String) {
        self.properties.push(SessionProperty::Identity(identity));
    }
}