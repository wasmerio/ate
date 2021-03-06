use super::crypto::*;
use super::signature::*;
use super::error::*;
use super::sink::*;
use super::meta::*;
use super::validator::*;
use super::compact::*;
use super::lint::*;
use super::session::*;
use super::transform::*;
use super::plugin::*;
use super::event::*;
use bytes::Bytes;
use fxhash::FxHashMap;

#[derive(Debug)]
pub struct TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    root_keys: FxHashMap<Hash, PublicKey>,
    signature_plugin: SignaturePlugin<M>,
}

impl<M> TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    pub fn new() -> TreeAuthorityPlugin<M> {
        TreeAuthorityPlugin {
            root_keys: FxHashMap::default(),
            signature_plugin: SignaturePlugin::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(&mut self, key: &PublicKey)
    {
        self.root_keys.insert(key.hash(), key.clone());
    }
}

impl<M> EventSink<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn feed(&mut self, meta: &MetadataExt<M>, data_hash: &Option<Hash>) -> Result<(), SinkError>
    {
        self.signature_plugin.feed(meta, data_hash)?;
        Ok(())
    }
}

impl<M> EventValidator<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn validate(&self, validation_data: &ValidationData<M>) -> Result<ValidationResult, ValidationError>
    {
        // We need to check all the signatures are valid
        self.signature_plugin.validate(validation_data)?;

        // If it has data then we need to check it - otherwise we ignore it
        let data_hash = match validation_data.data_hash {
            Some(a) => a,
            None => { return Ok(ValidationResult::Abstain); },
        };
        
        let verified_signatures = match self.signature_plugin.get_verified_signatures(&data_hash) {
            Some(a) => a,
            None => { return Err(ValidationError::NoSignatures); },
        };
        
        // If its got a root key attached to it then we are all good
        for hash in verified_signatures.iter() {
            if self.root_keys.contains_key(hash) {
                return Ok(ValidationResult::Allow);
            }
        }
        
        // If we get this far then any data events must be denied
        // as all the other possible routes for it to be excepted have already passed
        Err(ValidationError::Detached)
    }
}

impl<M> EventCompactor<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn clone_prepare(&self) -> Box<dyn EventCompactor<M>> {
        Box::new(IndecisiveCompactor::default())
    }

    fn relevance(&mut self, evt: &EventEntryExt<M>) -> EventRelevance {
        match self.signature_plugin.relevance(evt) {
            EventRelevance::Abstain => {},
            r => { return r; },
        }
        EventRelevance::Abstain
    }
}

impl<M> EventMetadataLinter<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata,
{
    fn metadata_lint_many(&self, data_hashes: &Vec<Hash>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();

        let mut other = self.signature_plugin.metadata_lint_many(data_hashes, session)?;
        ret.append(&mut other);

        Ok(ret)
    }

    fn metadata_lint_event(&self, data_hash: &Option<Hash>, meta: &MetadataExt<M>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();

        let mut no_auth_metadata = true;
        let mut no_auth = true;
        if let Some(auth) = meta.get_authorization() {
            no_auth_metadata = false;
            for write_hash in auth.allow_write.iter() {
                no_auth = false;
                if self.signature_plugin.has_public_key(&write_hash) == false
                {                    
                    // Make sure we actually own the key that it wants to write with
                    if let None = session.properties
                        .iter()
                        .filter_map(|p| {
                            match p {
                                SessionProperty::WriteKey(w) => {
                                    if w.hash() == *write_hash {
                                        Some(w)
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        })
                        .next()
                    {
                        // We could not find the write key!
                        return Err(LintError::MissingWriteKey(write_hash.clone()));
                    }
                }
            }
        }

        if data_hash.is_some() && no_auth == true
        {
            // This record has no authorization
            if no_auth_metadata {
                return match meta.get_data_key() {
                    Some(key) => Err(LintError::MissingAuthorizationMetadata(key)),
                    None => Err(LintError::MissingAuthorizationMetadataOrphan)
                };
            } else {
                return match meta.get_data_key() {
                    Some(key) => Err(LintError::NoAuthorization(key)),
                    None => Err(LintError::NoAuthorizationOrphan)
                };
            }
        }
        
        // Now run the signature plugin
        ret.extend(self.signature_plugin.metadata_lint_event(data_hash, meta, session)?);

        // We are done
        Ok(ret)
    }
}

impl<M> EventDataTransformer<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn data_as_underlay(&self, meta: &mut MetadataExt<M>, with: Bytes) -> Result<Bytes, TransformError> {
        let with = self.signature_plugin.data_as_underlay(meta, with)?;
        Ok(with)
    }

    fn data_as_overlay(&self, meta: &mut MetadataExt<M>, with: Bytes) -> Result<Bytes, TransformError> {
        let with = self.signature_plugin.data_as_overlay(meta, with)?;
        Ok(with)
    }
}

impl<M> EventPlugin<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata,
{
    fn rebuild(&mut self, data: &Vec<EventEntryExt<M>>) -> Result<(), SinkError>
    {
        self.signature_plugin.rebuild(data)?;
        Ok(())
    }
}