#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use super::crypto::*;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum CoreSession
{
    None,
    Secret
    {
        id: u64,
        key: EncryptKey,
    }
}

impl Default for CoreSession {
    fn default() -> Self {
        CoreSession::None
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Session<S>
{
    pub core: Vec<CoreSession>,
    pub other: S,
}