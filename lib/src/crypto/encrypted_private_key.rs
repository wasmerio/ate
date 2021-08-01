#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct EncryptedPrivateKey {
    pk: PublicSignKey,
    ek_hash: AteHash,
    sk_iv: InitializationVector,
    sk_encrypted: Vec<u8>
}

impl EncryptedPrivateKey
{
    #[allow(dead_code)]
    pub fn generate(encrypt_key: &EncryptKey) -> EncryptedPrivateKey {
        let pair = PrivateSignKey::generate(encrypt_key.size());
        EncryptedPrivateKey::from_pair(&pair, encrypt_key)
    }

    #[allow(dead_code)]
    pub fn from_pair(pair: &PrivateSignKey, encrypt_key: &EncryptKey) -> EncryptedPrivateKey {
        let sk = pair.sk();
        let sk = encrypt_key.encrypt(&sk[..]);
        
        EncryptedPrivateKey {
            pk: pair.as_public_key(),
            ek_hash: encrypt_key.hash(),
            sk_iv: sk.iv,
            sk_encrypted: sk.data,
        }
    }

    #[allow(dead_code)]
    pub fn as_private_key(&self, key: &EncryptKey) -> PrivateSignKey {
        let data = key.decrypt(&self.sk_iv, &self.sk_encrypted[..]);
        match &self.pk {
            PublicSignKey::Falcon512 { pk } => {
                PrivateSignKey::Falcon512 {
                    pk: pk.clone(),
                    sk: data,
                }
            },
            PublicSignKey::Falcon1024{ pk } => {
                PrivateSignKey::Falcon1024 {
                    pk: pk.clone(),
                    sk: data,
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicSignKey {
        self.pk.clone()
    }

    #[allow(dead_code)]
    pub fn pk_hash(&self) -> AteHash {
        self.pk.hash()
    }

    #[allow(dead_code)]
    pub(crate) fn double_hash(&self) -> DoubleHash {
        DoubleHash::from_hashes(&self.pk_hash(), &self.ek_hash)
    }
}