#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use fxhash::FxHashMap;
use serde::{Serialize, Deserialize};
use std::{io::ErrorKind, marker::PhantomData};
use std::result::Result;
use crate::spec::SerializationFormat;

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    format: SerializationFormat,
    ek_hash: AteHash,
    sd_iv: InitializationVector,
    sd_encrypted: Vec<u8>,
    #[serde(skip)]
    _marker: std::marker::PhantomData<T>,
}

impl<T> EncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    pub fn new(encrypt_key: &PublicEncryptKey, data: T) -> Result<EncryptedSecureData<T>, std::io::Error> {
        let format = SerializationFormat::Bincode;
        let data = match format.serialize(&data) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        };
        let result = encrypt_key.encrypt(&data[..]);
        
        Ok(
            EncryptedSecureData {
                format,
                ek_hash: encrypt_key.hash(),
                sd_iv: result.iv,
                sd_encrypted: result.data,
                _marker: PhantomData,
            }
        )
    }

    pub fn unwrap(&self, key: &PrivateEncryptKey) -> Result<T, std::io::Error> {
        let data = key.decrypt(&self.sd_iv, &self.sd_encrypted[..])?;
        Ok(match self.format.deserialize(&data[..]) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        })
    }

    pub fn ek_hash(&self) -> AteHash {
        self.ek_hash
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultiEncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    format: SerializationFormat,
    members: FxHashMap<String, EncryptedSecureData<EncryptKey>>,
    metadata: FxHashMap<String, String>,
    sd_iv: InitializationVector,
    sd_encrypted: Vec<u8>,
    #[serde(skip)]
    _marker2: std::marker::PhantomData<T>,
}

impl<T> MultiEncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    pub fn new(encrypt_key: &PublicEncryptKey, meta: String, data: T) -> Result<MultiEncryptedSecureData<T>, std::io::Error> {
        let format = SerializationFormat::Bincode;
        let shared_key = EncryptKey::generate(encrypt_key.size());
        
        let index = encrypt_key.hash().to_hex_string();
        let mut members = FxHashMap::default();
        members.insert(index.clone(), EncryptedSecureData::new(encrypt_key, shared_key)?);
        let mut metadata = FxHashMap::default();
        metadata.insert(index, meta);

        let data = match format.serialize(&data) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        };
        let result = shared_key.encrypt(&data[..]);
        
        Ok(
            MultiEncryptedSecureData {
                format,
                members,
                metadata,
                sd_iv: result.iv,
                sd_encrypted: result.data,
                _marker2: PhantomData,
            }
        )
    }

    pub fn unwrap(&self, key: &PrivateEncryptKey) -> Result<Option<T>, std::io::Error> {
        Ok(
            match self.members.get(&key.hash().to_hex_string()) {
                Some(a) => {
                    let shared_key = a.unwrap(key)?;
                    let data = shared_key.decrypt(&self.sd_iv, &self.sd_encrypted[..]);
                    Some(match self.format.deserialize::<T>(&data[..]) {
                        Ok(a) => a,
                        Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
                    })
                },
                None => None
            }
        )
    }

    pub fn add(&mut self, encrypt_key: &PublicEncryptKey, meta: String, referrer: &PrivateEncryptKey) -> Result<bool, std::io::Error> {
        match self.members.get(&referrer.hash().to_hex_string()) {
            Some(a) => {
                let shared_key = a.unwrap(referrer)?;
                let index = encrypt_key.hash().to_hex_string();
                self.members.insert(index.clone(), EncryptedSecureData::new(encrypt_key, shared_key)?);
                self.metadata.insert(index, meta);
                Ok(true)
            },
            None => Ok(false)
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