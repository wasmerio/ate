#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use std::{io::ErrorKind};
use std::result::Result;
use pqcrypto_falcon::falcon512;
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey as PQCryptoPublicKey};
use pqcrypto_traits::sign::SecretKey as PQCryptoSecretKey;
use crate::utils::vec_as_base64;
use crate::utils::vec_from_base64;

use super::*;

/// Private keys provide the ability to sign records within the
/// redo log chain-of-trust, these inserts records with associated
/// public keys embedded within teh cahin allow
/// records/events stored within the ATE redo log to have integrity
/// without actually being able to read the records themselves. This
/// attribute allows a chain-of-trust to be built without access to
/// the data held within of chain. Asymetric crypto in ATE uses the
/// leading candidates from NIST that provide protection against
/// quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PrivateSignKey {
    Falcon512 {
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        pk: Vec<u8>,
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        sk: Vec<u8>,
    },
    Falcon1024 {
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        pk: Vec<u8>,
        #[serde(serialize_with = "vec_as_base64", deserialize_with = "vec_from_base64")]
        sk: Vec<u8>,
    },
}

impl PrivateSignKey
{
    #[allow(dead_code)]
    pub fn generate(size: KeySize) -> PrivateSignKey {
        match size {
            KeySize::Bit128 | KeySize::Bit192 => {
                let (pk, sk) = falcon512::keypair();
                PrivateSignKey::Falcon512 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit256 => {
                let (pk, sk) = falcon1024::keypair();
                PrivateSignKey::Falcon1024 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicSignKey {
        match &self {
            PrivateSignKey::Falcon512 { sk: _, pk } => {
                PublicSignKey::Falcon512 {
                    pk: pk.clone(),
                }
            },
            PrivateSignKey::Falcon1024 { sk: _, pk } => {
                PublicSignKey::Falcon1024 {
                    pk: pk.clone(),
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> AteHash {
        match &self {
            PrivateSignKey::Falcon512 { pk, sk: _ } => AteHash::from_bytes(&pk[..]),
            PrivateSignKey::Falcon1024 { pk, sk: _ } => AteHash::from_bytes(&pk[..]),
        }
    }

    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PrivateSignKey::Falcon512 { pk, sk: _ } => pk.clone(),
            PrivateSignKey::Falcon1024 { pk, sk: _ } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sk(&self) -> Vec<u8> { 
        match &self {
            PrivateSignKey::Falcon512 { pk: _, sk } => sk.clone(),
            PrivateSignKey::Falcon1024 { pk: _, sk } => sk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let ret = match &self {
            PrivateSignKey::Falcon512 { pk: _, sk } => {
                let sk = match falcon512::SecretKey::from_bytes(&sk[..]) {
                    Ok(sk) => sk,
                    Err(err) => { return Result::Err(std::io::Error::new(ErrorKind::Other, format!("Failed to decode the secret key ({}).", err))); },
                };
                let sig = falcon512::detached_sign(data, &sk);
                Vec::from(sig.as_bytes())
            },
            PrivateSignKey::Falcon1024 { pk: _, sk } => {
                let sk = match falcon1024::SecretKey::from_bytes(&sk[..]) {
                    Ok(sk) => sk,
                    Err(err) => { return Result::Err(std::io::Error::new(ErrorKind::Other, format!("Failed to decode the secret key ({}).", err))); },
                };
                let sig = falcon1024::detached_sign(data, &sk);
                Vec::from(sig.as_bytes())
            },
        };
        
        Ok(ret)
    }

    pub fn size(&self) -> KeySize {
        match &self {
            PrivateSignKey::Falcon512 { pk: _, sk: _ } => KeySize::Bit192,
            PrivateSignKey::Falcon1024 { pk: _, sk: _ } => KeySize::Bit256,
        }
    }
}

impl std::fmt::Display
for PrivateSignKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivateSignKey::Falcon512 { pk: _, sk: _ } => write!(f, "falcon512:pk:{}+sk", self.hash()),
            PrivateSignKey::Falcon1024 { pk: _, sk: _ } => write!(f, "falcon1024:pk:{}+sk", self.hash()),
        }
    }
}

/// Public key which is one side of a private key. Public keys allow
/// records/events stored within the ATE redo log to have integrity
/// without actually being able to read the records themselves. This
/// attribute allows a chain-of-trust to be built without access to
/// the data held within of chain. Asymetric crypto in ATE uses the
/// leading candidates from NIST that provide protection against
/// quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PublicSignKey {
    Falcon512 {
        pk: Vec<u8>,
    },
    Falcon1024 {
        pk: Vec<u8>,
    }
}

impl PublicSignKey
{
    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PublicSignKey::Falcon512 { pk } => pk.clone(),
            PublicSignKey::Falcon1024 { pk } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> AteHash {
        match &self {
            PublicSignKey::Falcon512 { pk } => AteHash::from_bytes(&pk[..]),
            PublicSignKey::Falcon1024 { pk } => AteHash::from_bytes(&pk[..]),
        }
    }
    
    #[allow(dead_code)]
    pub fn verify(&self, data: &[u8], sig: &[u8]) -> Result<bool, pqcrypto_traits::Error> {
        let ret = match &self {
            PublicSignKey::Falcon512 { pk } => {
                let pk = falcon512::PublicKey::from_bytes(&pk[..])?;
                let sig = falcon512::DetachedSignature::from_bytes(sig)?;
                falcon512::verify_detached_signature(&sig, data, &pk).is_ok()
            },
            PublicSignKey::Falcon1024 { pk } => {
                let pk = falcon1024::PublicKey::from_bytes(&pk[..])?;
                let sig = falcon1024::DetachedSignature::from_bytes(sig)?;
                falcon1024::verify_detached_signature(&sig, data, &pk).is_ok()
            }
        };
        
        Ok(ret)
    }
}

impl std::fmt::Display
for PublicSignKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicSignKey::Falcon512 { pk: _ } => write!(f, "falcon512:pk:{}", self.hash()),
            PublicSignKey::Falcon1024 { pk: _ } => write!(f, "falcon1024:pk:{}", self.hash()),
        }
    }
}