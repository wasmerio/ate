use super::meta::*;
use super::crypto::*;
use super::event::*;
use tokio::io::Result;

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
    pub meta: &'a Metadata<M>,
    pub data_hash: Option<super::crypto::Hash>,
}

impl<'a, M> ValidationData<'a, M>
where M: OtherMetadata
{
    pub fn from_event_entry(evt: &'a EventEntry<M>) -> ValidationData<'a, M> {
        ValidationData {
            meta: &evt.meta,
            data_hash: evt.data_hash.clone(),
        }
    }

    pub fn from_event(evt: &'a Event<M>) -> ValidationData<'a, M> {
        ValidationData {
            meta: &evt.meta,
            data_hash: evt.body_hash.clone(),
        }
    }
}

pub trait EventValidator<M>
where M: OtherMetadata
{
    fn validate(&self, _evt: &ValidationData<M>) -> Result<ValidationResult> {
        Ok(ValidationResult::Abstain)
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
    fn validate(&self, _evt: &ValidationData<M>) -> Result<ValidationResult>
    {
        Ok(ValidationResult::Allow)
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
    fn validate(&self, _evt: &ValidationData<M>) -> Result<ValidationResult>
    {
        Ok(ValidationResult::Allow)
    }
}

impl<M> Metadata<M>
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