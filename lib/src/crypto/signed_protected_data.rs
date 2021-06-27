#[allow(unused_imports)]
use log::{info, error, debug};
use serde::{Serialize, Deserialize};
use std::io::ErrorKind;
use std::result::Result;
use crate::spec::SerializationFormat;

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedProtectedData<T>
where T: serde::Serialize
{
    format: SerializationFormat,
    sig: Vec<u8>,
    data: T,
}

impl<T> SignedProtectedData<T>
where T: serde::Serialize
{
    pub fn new(sign_key: &PrivateSignKey, data: T) -> Result<SignedProtectedData<T>, std::io::Error> {
        let format = SerializationFormat::Bincode;
        let binary_data = match format.serialize(&data) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        };
        let sig = sign_key.sign(&binary_data[..])?;
        
        Ok(
            SignedProtectedData {
                format,
                sig,
                data,
            }
        )
    }

    pub fn verify(&self, key: &PublicSignKey) -> Result<bool, std::io::Error> {
        let binary_data = match self.format.serialize(&self.data) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        };
        match key.verify(&binary_data[..], &self.sig[..]) {
            Ok(a) => Ok(a),
            Err(err) => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
            }
        }
    }
}

impl<T> std::ops::Deref
for SignedProtectedData<T>
where T: serde::Serialize
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}