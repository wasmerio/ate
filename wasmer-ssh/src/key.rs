use serde::*;
use std::convert::TryInto;
use thrussh_keys::key::ed25519;
use thrussh_keys::key::KeyPair;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SshServerKey {
    Ed25519(Vec<u8>),
}

impl SshServerKey {
    pub fn generate_ed25519() -> SshServerKey {
        let key = KeyPair::generate_ed25519().unwrap();
        let key: SshServerKey = key.into();
        key
    }
}

impl Into<KeyPair> for SshServerKey {
    fn into(self) -> KeyPair {
        match self {
            SshServerKey::Ed25519(a) => KeyPair::Ed25519(ed25519::SecretKey {
                key: a.try_into().unwrap(),
            }),
        }
    }
}

impl From<KeyPair> for SshServerKey {
    fn from(key: KeyPair) -> SshServerKey {
        match key {
            KeyPair::Ed25519(a) => SshServerKey::Ed25519(a.key.to_vec()),
        }
    }
}
