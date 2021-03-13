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
{
    pub properties: Vec<SessionProperty>,
}