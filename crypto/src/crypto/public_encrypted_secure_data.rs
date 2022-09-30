use crate::spec::SerializationFormat;
use crate::utils::vec_deserialize;
use crate::utils::vec_serialize;
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::result::Result;
use std::{io::ErrorKind, marker::PhantomData};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicEncryptedSecureData<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    format: SerializationFormat,
    ek_hash: AteHash,
    sd_iv: InitializationVector,
    #[serde(serialize_with = "vec_serialize", deserialize_with = "vec_deserialize")]
    sd_encrypted: Vec<u8>,
    #[serde(skip)]
    _marker: std::marker::PhantomData<T>,
}

impl<T> PublicEncryptedSecureData<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    pub fn new(
        encrypt_key: &PublicEncryptKey,
        data: T,
    ) -> Result<PublicEncryptedSecureData<T>, std::io::Error> {
        let format = SerializationFormat::Bincode;
        let data = match format.serialize(&data) {
            Ok(a) => a,
            Err(err) => {
                return Err(std::io::Error::new(ErrorKind::Other, err.to_string()));
            }
        };
        let result = encrypt_key.encrypt(&data[..]);

        Ok(PublicEncryptedSecureData {
            format,
            ek_hash: encrypt_key.hash(),
            sd_iv: result.iv,
            sd_encrypted: result.data,
            _marker: PhantomData,
        })
    }

    pub fn unwrap(&self, key: &PrivateEncryptKey) -> Result<T, std::io::Error> {
        if key.hash() != self.ek_hash {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("The decryption key is not valid for this cipher data ({} vs {}).", key.hash(), self.ek_hash).as_str(),
            ));
        }
        let data = key.decrypt(&self.sd_iv, &self.sd_encrypted[..]).unwrap();
        Ok(match self.format.deserialize_ref(&data[..]) {
            Ok(a) => a,
            Err(err) => {
                return Err(std::io::Error::new(ErrorKind::Other, err.to_string()));
            }
        })
    }

    pub fn ek_hash(&self) -> AteHash {
        self.ek_hash
    }
}

impl<T> std::fmt::Display
for PublicEncryptedSecureData<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "secure_data(format={},ek_hash={},size={})", self.format, self.ek_hash, self.sd_encrypted.len())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultiEncryptedSecureData<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    format: SerializationFormat,
    members: FxHashMap<String, PublicEncryptedSecureData<EncryptKey>>,
    metadata: FxHashMap<String, String>,
    sd_iv: InitializationVector,
    sd_hash: AteHash,
    #[serde(serialize_with = "vec_serialize", deserialize_with = "vec_deserialize")]
    sd_encrypted: Vec<u8>,
    #[serde(skip)]
    _marker2: std::marker::PhantomData<T>,
}

impl<T> MultiEncryptedSecureData<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    pub fn new(
        encrypt_key: &PublicEncryptKey,
        meta: String,
        data: T,
    ) -> Result<MultiEncryptedSecureData<T>, std::io::Error> {
        let shared_key = EncryptKey::generate(encrypt_key.size());
        MultiEncryptedSecureData::new_ext(encrypt_key, shared_key, meta, data)
    }

    pub fn new_ext(
        encrypt_key: &PublicEncryptKey,
        shared_key: EncryptKey,
        meta: String,
        data: T,
    ) -> Result<MultiEncryptedSecureData<T>, std::io::Error> {
        let format = SerializationFormat::Bincode;

        let index = encrypt_key.hash().to_hex_string();
        let mut members = FxHashMap::default();
        members.insert(
            index.clone(),
            PublicEncryptedSecureData::new(encrypt_key, shared_key)?,
        );
        let mut metadata = FxHashMap::default();
        metadata.insert(index, meta);

        let data = match format.serialize(&data) {
            Ok(a) => a,
            Err(err) => {
                return Err(std::io::Error::new(ErrorKind::Other, err.to_string()));
            }
        };
        let result = shared_key.encrypt(&data[..]);
        let hash = AteHash::from_bytes_twice(&result.iv.bytes[..], &data[..]);

        Ok(MultiEncryptedSecureData {
            format,
            members,
            metadata,
            sd_iv: result.iv,
            sd_hash: hash,
            sd_encrypted: result.data,
            _marker2: PhantomData,
        })
    }

    pub fn unwrap(&self, key: &PrivateEncryptKey) -> Result<Option<T>, std::io::Error> {
        Ok(match self.members.get(&key.hash().to_hex_string()) {
            Some(a) => {
                let shared_key = a.unwrap(key)?;
                let data = shared_key.decrypt(&self.sd_iv, &self.sd_encrypted[..]);
                Some(match self.format.deserialize_ref::<T>(&data[..]) {
                    Ok(a) => a,
                    Err(err) => {
                        return Err(std::io::Error::new(ErrorKind::Other, err.to_string()));
                    }
                })
            }
            None => None,
        })
    }

    pub fn unwrap_shared(&self, shared_key: &EncryptKey) -> Result<Option<T>, std::io::Error> {
        let data = shared_key.decrypt(&self.sd_iv, &self.sd_encrypted[..]);
        let hash = AteHash::from_bytes_twice(&self.sd_iv.bytes[..], &data[..]);
        if hash != self.sd_hash {
            return Ok(None);
        }

        Ok(match self.format.deserialize::<T>(data) {
            Ok(a) => Some(a),
            Err(err) => {
                return Err(std::io::Error::new(ErrorKind::Other, err.to_string()));
            }
        })
    }

    pub fn add(
        &mut self,
        encrypt_key: &PublicEncryptKey,
        meta: String,
        referrer: &PrivateEncryptKey,
    ) -> Result<bool, std::io::Error> {
        match self.members.get(&referrer.hash().to_hex_string()) {
            Some(a) => {
                let shared_key = a.unwrap(referrer)?;
                let index = encrypt_key.hash().to_hex_string();
                self.members.insert(
                    index.clone(),
                    PublicEncryptedSecureData::new(encrypt_key, shared_key)?,
                );
                self.metadata.insert(index, meta);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn remove(&mut self, what: &AteHash) -> bool {
        let index = what.to_hex_string();
        let ret = self.members.remove(&index).is_some();
        self.metadata.remove(&index);
        ret
    }

    pub fn exists(&self, what: &AteHash) -> bool {
        let what = what.to_hex_string();
        self.members.contains_key(&what)
    }

    pub fn meta<'a>(&'a self, what: &AteHash) -> Option<&'a String> {
        let index = what.to_hex_string();
        self.metadata.get(&index)
    }

    pub fn meta_list<'a>(&'a self) -> impl Iterator<Item = &'a String> {
        self.metadata.values()
    }
}