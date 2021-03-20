use crate::crypto::EncryptedEncryptKeys;

use super::error::*;
use super::meta::*;
use super::lint::*;
use super::plugin::*;
use super::index::*;
use super::session::*;
use super::sink::*;
use super::transform::*;
use super::compact::*;
use super::validator::*;
use super::crypto::*;

use bytes::Bytes;
use fxhash::FxHashMap;

pub struct ConfidentialityPlugin {
}

impl ConfidentialityPlugin
{
    pub fn new() -> ConfidentialityPlugin
    {
        ConfidentialityPlugin {
        }
    }

    fn get_encrypt_key<'a>(keys: &EncryptedEncryptKeys, session: &Session) -> Option<EncryptKey>
    {
        let mut lookup = FxHashMap::default();
        for prop in session.properties.iter() {
            if let SessionProperty::ReadKey(key) = prop {
                lookup.insert(key.hash(), key.clone());
            }
        }

        for key in keys.eks.iter() {
            match lookup.get(&key.hash()) {
                Some(key) => {
                    return Some(key.clone());
                },
                _ => {}
            }
        }

        None
    }
}

impl Default
for ConfidentialityPlugin
{
    fn default() -> ConfidentialityPlugin
    {
        ConfidentialityPlugin::new()
    }
}

impl<M> EventMetadataLinter<M>
for ConfidentialityPlugin
where M: OtherMetadata,
{
    fn metadata_lint_event(&self, _meta: &MetadataExt<M>, _session: &Session)-> Result<Vec<CoreMetadata>, LintError> {
        let ret = Vec::new();
        Ok(ret)
    }
}

impl<M> EventSink<M>
for ConfidentialityPlugin
where M: OtherMetadata,
{
}

impl<M> EventIndexer<M>
for ConfidentialityPlugin
where M: OtherMetadata,
{
}

impl<M> EventDataTransformer<M>
for ConfidentialityPlugin
where M: OtherMetadata,
{
    #[allow(unused_variables)]
    fn data_as_underlay(&self, meta: &mut MetadataExt<M>, with: Bytes, session: &Session) -> Result<Bytes, TransformError>
    {
        let keys = match meta.get_confidentiality() {
            Some(a) => a,
            None => { return Ok(with) }
        };

        match ConfidentialityPlugin::get_encrypt_key(keys, session) {
            Some(key) => {
                let iv = meta.generate_iv();        
                let encrypted = key.encrypt_with_iv(&iv, &with[..])?;
                Ok(Bytes::from(encrypted))
            },
            None => {
                Err(TransformError::MissingReadKey)
            }
        }
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &mut MetadataExt<M>, with: Bytes, session: &Session) -> Result<Bytes, TransformError>
    {
        let keys = match meta.get_confidentiality() {
            Some(a) => a,
            None => { return Ok(with) }
        };

        match ConfidentialityPlugin::get_encrypt_key(keys, session) {
            Some(key) => {
                let iv = meta.get_iv()?;
                let decrypted = key.decrypt(&iv, &with[..])?;
                Ok(Bytes::from(decrypted))
            },
            None => {
                Err(TransformError::MissingReadKey)
            }
        }
    }
}

impl<M> EventCompactor<M>
for ConfidentialityPlugin
where M: OtherMetadata,
{
}

impl<M> EventValidator<M>
for ConfidentialityPlugin
where M: OtherMetadata,
{
}

impl<M> EventPlugin<M>
for ConfidentialityPlugin
where M: OtherMetadata,
{
}