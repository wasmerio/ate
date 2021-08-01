#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use std::result::Result;

use super::*;

/// Encrypt key material is used to transform an encryption key using
/// derivation which should allow encryption keys to be changed without
/// having to decrypt and reencrypt the data itself.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DerivedEncryptKey
{
    pub(crate) inner: EncryptResult
}

impl DerivedEncryptKey
{
    pub fn new(key: &EncryptKey) -> DerivedEncryptKey {
        let inner = EncryptKey::generate(key.size());
        DerivedEncryptKey {
            inner: key.encrypt(inner.value())
        }
    }

    pub fn reverse(key: &EncryptKey, inner: &EncryptKey) -> DerivedEncryptKey {
        DerivedEncryptKey {
            inner: key.encrypt(inner.value())
        }
    }

    pub fn transmute(&self, key: &EncryptKey) -> Result<EncryptKey, std::io::Error>
    {
        // Decrypt the derived key
        let bytes = key.decrypt(&self.inner.iv, &self.inner.data[..]);
        Ok(EncryptKey::from_bytes(&bytes[..])?)
    }

    pub fn transmute_private(&self, key: &PrivateEncryptKey) -> Result<EncryptKey, std::io::Error>
    {
        // Decrypt the derived key
        let bytes = key.decrypt(&self.inner.iv, &self.inner.data[..])?;
        Ok(EncryptKey::from_bytes(&bytes[..])?)
    }

    pub fn change(&mut self, old: &EncryptKey, new: &EncryptKey) -> Result<(), std::io::Error>
    {
        // First derive the key, then replace the inner with a newly encrypted value
        let inner = self.transmute(old)?;
        self.inner = new.encrypt(inner.value());
        Ok(())
    }

    pub fn change_private(&mut self, old: &PrivateEncryptKey, new: &PublicEncryptKey) -> Result<(), std::io::Error>
    {
        // First derive the key, then replace the inner with a newly encrypted value
        let inner = self.transmute_private(old)?;
        self.inner = new.encrypt(inner.value());
        Ok(())
    }
}