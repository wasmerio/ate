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

#[derive(Debug)]
pub struct TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    root_keys: Vec<Hash>,
    signature_plugin: SignaturePlugin<M>,
}

impl<M> TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    pub fn new() -> TreeAuthorityPlugin<M> {
        TreeAuthorityPlugin {
            root_keys: Vec::new(),
            signature_plugin: SignaturePlugin::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(&mut self, key: &PublicKey)
    {
        self.root_keys.push(key.hash());
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
    fn validate(&self, validation_data: &ValidationData<M>) -> ValidationResult {
        let sig_val = match self.signature_plugin.validate(validation_data) {
            ValidationResult::Deny => { return ValidationResult::Deny; },
            r => r,
        };
        
        // If we get this far then any data events must be denied
        match validation_data.data_hash {
            Some(_) => ValidationResult::Deny,
            None => sig_val,
        }
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
    fn metadata_lint_many(&self, data_hashes: &Vec<Hash>, session: &Session) -> Result<Vec<CoreMetadata>, std::io::Error>
    {
        let mut ret = Vec::new();
        let mut other = self.signature_plugin.metadata_lint_many(data_hashes, session)?;
        ret.append(&mut other);
        Ok(ret)
    }

    fn metadata_lint_event(&self, data_hash: &Option<Hash>, meta: &mut MetadataExt<M>, session: &Session)
    {
        self.signature_plugin.metadata_lint_event(data_hash, meta, session);
    }

    fn metadata_trim(&self, meta: &mut MetadataExt<M>)
    {
        self.signature_plugin.metadata_trim(meta);
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