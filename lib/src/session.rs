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