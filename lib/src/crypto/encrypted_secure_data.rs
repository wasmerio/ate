#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use std::{io::ErrorKind, marker::PhantomData};
use std::result::Result;
use crate::spec::SerializationFormat;
use crate::utils::vec_serialize;
use crate::utils::vec_deserialize;

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    format: SerializationFormat,
    ek_hash: AteHash,
    sd_iv: InitializationVector,
    #[serde(serialize_with = "vec_serialize", deserialize_with = "vec_deserialize")]
    sd_encrypted: Vec<u8>,
    #[serde(skip)]
    _marker: std::marker::PhantomData<T>,
}

impl<T> EncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    pub fn new(encrypt_key: &EncryptKey, data: T) -> Result<EncryptedSecureData<T>, std::io::Error> {
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

    pub fn unwrap(&self, key: &EncryptKey) -> Result<T, std::io::Error> {
        let data = key.decrypt(&self.sd_iv, &self.sd_encrypted[..]);
        Ok(match self.format.deserialize(&data[..]) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        })
    }

    pub fn ek_hash(&self) -> AteHash {
        self.ek_hash
    }
}