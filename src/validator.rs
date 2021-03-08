use super::meta::*;
use super::crypto::*;
use super::event::*;
use super::signature::MetaSignature;
use super::crypto::Hash;
use super::error::*;

#[derive(Debug)]
pub enum ValidationResult {
    Allow,
    #[allow(dead_code)]
    Abstain,
}

pub struct ValidationData<'a>
{
    pub meta_hash: Hash,
    pub meta: &'a Metadata,
    pub data_hash: Option<Hash>,
}

impl<'a> ValidationData<'a>
{
    pub fn from_event_entry(evt: &'a EventEntryExt) -> ValidationData<'a> {
        ValidationData {
            meta_hash: evt.meta_hash,
            meta: &evt.meta,
            data_hash: evt.data_hash.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn from_event(evt: &'a EventRawPlus) -> ValidationData<'a> {
        ValidationData {
            meta_hash: evt.meta_hash.clone(),
            meta: &evt.inner.meta,
            data_hash: evt.inner.data_hash.clone(),
        }
    }
}

pub trait EventValidator
{
    fn validate(&self, _validation_data: &ValidationData) -> Result<ValidationResult, ValidationError> {
        Ok(ValidationResult::Abstain)
    }
}

#[derive(Default)]
pub struct RubberStampValidator {   
}

impl EventValidator
for RubberStampValidator
{
    #[allow(unused_variables)]
    fn validate(&self, _validation_data: &ValidationData) -> Result<ValidationResult, ValidationError>
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

impl EventValidator
for StaticSignatureValidator
{
    #[allow(unused_variables)]
    fn validate(&self, _validation_data: &ValidationData) -> Result<ValidationResult, ValidationError>
    {
        Ok(ValidationResult::Allow)
    }
}

impl Metadata
{
    #[allow(dead_code)]
    pub fn add_signature(&mut self, _sig: MetaSignature) {
    }

    #[allow(dead_code)]
    pub fn get_signature(&self) -> Option<MetaSignature> {
        None
    }
}