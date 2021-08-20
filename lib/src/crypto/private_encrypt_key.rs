#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use std::result::Result;
use pqcrypto_ntru::ntruhps2048509 as ntru128;
use pqcrypto_ntru::ntruhps2048677 as ntru192;
use pqcrypto_ntru::ntruhps4096821 as ntru256;
use pqcrypto_traits::kem::*;
use crate::utils::vec_as_base64;
use crate::utils::vec_from_base64;

use super::*;

/// Private encryption keys provide the ability to decrypt a secret
/// that was encrypted using a Public Key - this capability is
/// useful for key-exchange and trust validation in the crypto chain.
/// Asymetric crypto in ATE uses the leading candidates from NIST
/// that provide protection against quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PrivateEncryptKey {
    Ntru128 {
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        pk: Vec<u8>,
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        sk: Vec<u8>,
    },
    Ntru192 {
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        pk: Vec<u8>,
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        sk: Vec<u8>,
    },
    Ntru256 {
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        pk: Vec<u8>,
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        sk: Vec<u8>,
    },
}

impl PrivateEncryptKey
{
    #[allow(dead_code)]
    pub fn generate(size: KeySize) -> PrivateEncryptKey {
        match size {
            KeySize::Bit128 => {
                let (pk, sk) = ntru128::keypair();
                PrivateEncryptKey::Ntru128 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit192 => {
                let (pk, sk) = ntru192::keypair();
                PrivateEncryptKey::Ntru192 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit256 => {
                let (pk, sk) = ntru256::keypair();
                PrivateEncryptKey::Ntru256 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicEncryptKey {
        match &self {
            PrivateEncryptKey::Ntru128 { sk: _, pk } => {
                PublicEncryptKey::Ntru128 {
                    pk: pk.clone(),
                }
            },
            PrivateEncryptKey::Ntru192 { sk: _, pk } => {
                PublicEncryptKey::Ntru192 {
                    pk: pk.clone(),
                }
            },
            PrivateEncryptKey::Ntru256 { sk: _, pk } => {
                PublicEncryptKey::Ntru256 {
                    pk: pk.clone(),
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> AteHash {
        match &self {
            PrivateEncryptKey::Ntru128 { pk, sk: _ } => AteHash::from_bytes(&pk[..]),
            PrivateEncryptKey::Ntru192 { pk, sk: _ } => AteHash::from_bytes(&pk[..]),
            PrivateEncryptKey::Ntru256 { pk, sk: _ } => AteHash::from_bytes(&pk[..]),
        }
    }

    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PrivateEncryptKey::Ntru128 { pk, sk: _ } => pk.clone(),
            PrivateEncryptKey::Ntru192 { pk, sk: _ } => pk.clone(),
            PrivateEncryptKey::Ntru256 { pk, sk: _ } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sk(&self) -> Vec<u8> { 
        match &self {
            PrivateEncryptKey::Ntru128 { pk: _, sk } => sk.clone(),
            PrivateEncryptKey::Ntru192 { pk: _, sk } => sk.clone(),
            PrivateEncryptKey::Ntru256 { pk: _, sk } => sk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn decapsulate(&self, iv: &InitializationVector) -> Option<EncryptKey> {
        match &self {
            PrivateEncryptKey::Ntru128 { pk: _, sk } => {
                if iv.bytes.len() != ntru128::ciphertext_bytes() { return None; }
                let ct = ntru128::Ciphertext::from_bytes(&iv.bytes[..]).unwrap();
                let sk = ntru128::SecretKey::from_bytes(&sk[..]).unwrap();
                let ss = ntru128::decapsulate(&ct, &sk);
                Some(EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit128))
            },
            PrivateEncryptKey::Ntru192 { pk: _, sk } => {
                if iv.bytes.len() != ntru192::ciphertext_bytes() { return None; }
                let ct = ntru192::Ciphertext::from_bytes(&iv.bytes[..]).unwrap();
                let sk = ntru192::SecretKey::from_bytes(&sk[..]).unwrap();
                let ss = ntru192::decapsulate(&ct, &sk);
                Some(EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit192))
            },
            PrivateEncryptKey::Ntru256 { pk: _, sk } => {
                if iv.bytes.len() != ntru256::ciphertext_bytes() { return None; }
                let ct = ntru256::Ciphertext::from_bytes(&iv.bytes[..]).unwrap();
                let sk = ntru256::SecretKey::from_bytes(&sk[..]).unwrap();
                let ss = ntru256::decapsulate(&ct, &sk);
                Some(EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit256))
            },
        }
    }
    
    pub fn decrypt(&self, iv: &InitializationVector, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let ek = match self.decapsulate(iv) {
            Some(a) => a,
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "The encryption key could not be decapsulated from the initialization vector."));
            }
        };
        Ok(ek.decrypt(iv, data))
    }

    pub fn size(&self) -> KeySize {
        match &self {
            PrivateEncryptKey::Ntru128 { pk: _, sk: _ } => KeySize::Bit128,
            PrivateEncryptKey::Ntru192 { pk: _, sk: _ } => KeySize::Bit192,
            PrivateEncryptKey::Ntru256 { pk: _, sk: _ } => KeySize::Bit256,
        }
    }
}

impl std::fmt::Display
for PrivateEncryptKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivateEncryptKey::Ntru128 { pk: _, sk: _ } => write!(f, "ntru128:pk:{}+sk", self.hash()),
            PrivateEncryptKey::Ntru192 { pk: _, sk: _ } => write!(f, "ntru192:pk:{}+sk", self.hash()),
            PrivateEncryptKey::Ntru256 { pk: _, sk: _ } => write!(f, "ntru256:pk:{}+sk", self.hash()),
        }
    }
}

/// Public encryption keys provide the ability to encrypt a secret
/// without the ability to decrypt it yourself - this capability is
/// useful for key-exchange and trust validation in the crypto chain.
/// Asymetric crypto in ATE uses the leading candidates from NIST
/// that provide protection against quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PublicEncryptKey {
    Ntru128 {
        pk: Vec<u8>,
    },
    Ntru192 {
        pk: Vec<u8>,
    },
    Ntru256 {
        pk: Vec<u8>,
    }
}

impl PublicEncryptKey
{
    pub fn from_bytes(bytes: Vec<u8>) -> Option<PublicEncryptKey>
    {
        match bytes.len() {
            a if a == ntru128::public_key_bytes() => Some(PublicEncryptKey::Ntru128 { pk: bytes }),
            a if a == ntru192::public_key_bytes() => Some(PublicEncryptKey::Ntru192 { pk: bytes }),
            a if a == ntru256::public_key_bytes() => Some(PublicEncryptKey::Ntru256 { pk: bytes }),
            _ => None,
        }
    }

    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PublicEncryptKey::Ntru128 { pk } => pk.clone(),
            PublicEncryptKey::Ntru192 { pk } => pk.clone(),
            PublicEncryptKey::Ntru256 { pk } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> AteHash {
        match &self {
            PublicEncryptKey::Ntru128 { pk } => AteHash::from_bytes(&pk[..]),
            PublicEncryptKey::Ntru192 { pk } => AteHash::from_bytes(&pk[..]),
            PublicEncryptKey::Ntru256 { pk } => AteHash::from_bytes(&pk[..]),
        }
    }

    #[allow(dead_code)]
    pub fn encapsulate(&self) -> (InitializationVector, EncryptKey) {
        match &self {
            PublicEncryptKey::Ntru128 { pk } => {
                let pk = ntru128::PublicKey::from_bytes(&pk[..]).unwrap();
                let (ss, ct) = ntru128::encapsulate(&pk);
                let iv = InitializationVector::from(ct.as_bytes());
                (iv, EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit128))
            },
            PublicEncryptKey::Ntru192 { pk } => {
                let pk = ntru192::PublicKey::from_bytes(&pk[..]).unwrap();
                let (ss, ct) = ntru192::encapsulate(&pk);
                let iv = InitializationVector::from(ct.as_bytes());
                (iv, EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit192))
            },
            PublicEncryptKey::Ntru256 { pk } => {
                let pk = ntru256::PublicKey::from_bytes(&pk[..]).unwrap();
                let (ss, ct) = ntru256::encapsulate(&pk);
                let iv = InitializationVector::from(ct.as_bytes());
                (iv, EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit256))
            },
        }
    }

    pub fn encrypt(&self, data: &[u8]) -> EncryptResult {
        let (iv, ek) = self.encapsulate();
        let data = ek.encrypt_with_iv(&iv, data);
        EncryptResult {
            iv,
            data,
        }
    }

    pub fn size(&self) -> KeySize {
        match &self {
            PublicEncryptKey::Ntru128 { pk: _ } => KeySize::Bit128,
            PublicEncryptKey::Ntru192 { pk: _ } => KeySize::Bit192,
            PublicEncryptKey::Ntru256 { pk: _ } => KeySize::Bit256,
        }
    }
}

impl std::fmt::Display
for PublicEncryptKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicEncryptKey::Ntru128 { pk: _ } => write!(f, "ntru128:pk:{}", self.hash()),
            PublicEncryptKey::Ntru192 { pk: _ } => write!(f, "ntru192:pk:{}", self.hash()),
            PublicEncryptKey::Ntru256 { pk: _ } => write!(f, "ntru256:pk:{}", self.hash()),
        }
    }
}