use super::meta::*;
use super::crypto::*;
use super::event::*;
use super::signature::MetaSignature;
use super::crypto::Hash;

#[derive(Debug)]
pub enum ValidationResult {
    Allow,
    #[allow(dead_code)]
    Abstain,
    #[allow(dead_code)]
    Deny,
}

pub struct ValidationData<'a, M>
where M: OtherMetadata
{
    pub meta: &'a MetadataExt<M>,
    pub data_hash: Option<Hash>,
}

impl<'a, M> ValidationData<'a, M>
where M: OtherMetadata
{
    pub fn from_event_entry(evt: &'a EventEntryExt<M>) -> ValidationData<'a, M> {
        ValidationData {
            meta: &evt.meta,
            data_hash: evt.data_hash.clone(),
        }
    }

    pub fn from_event(evt: &'a EventRaw<M>) -> ValidationData<'a, M> {
        ValidationData {
            meta: &evt.meta,
            data_hash: evt.data_hash.clone(),
        }
    }
}

pub trait EventValidator<M>
where M: OtherMetadata
{
    fn validate(&self, _validation_data: &ValidationData<M>) -> ValidationResult {
        ValidationResult::Abstain
    }
}

#[derive(Default)]
pub struct RubberStampValidator {   
}

impl<M> EventValidator<M>
for RubberStampValidator
where M: OtherMetadata
{
    #[allow(unused_variables)]
    fn validate(&self, _validation_data: &ValidationData<M>) -> ValidationResult
    {
        ValidationResult::Allow
    }
}

pub struct StaticSignatureValidator {
    #[allow(dead_code)]
    pk: PublicKey,
}

impl StaticSignatureValidator
{
    #[allow(dead_code)]
    pub fn new(key: &PublicKey) -> StaticSignatureValidator {
        StaticSignatureValidator {
            pk: key.clone(),
        }
    }
}

impl<M> EventValidator<M>
for StaticSignatureValidator
where M: OtherMetadata
{
    #[allow(unused_variables)]
    fn validate(&self, _validation_data: &ValidationData<M>) -> ValidationResult
    {
        ValidationResult::Allow
    }
}

impl<M> MetadataExt<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn add_signature(&mut self, _sig: MetaSignature) {
    }

    #[allow(dead_code)]
    pub fn get_signature(&self) -> Option<MetaSignature> {
        None
    }
}