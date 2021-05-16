use std::sync::Arc;

use super::meta::*;
use super::crypto::*;
use super::event::*;
use super::signature::MetaSignature;
use super::error::*;
use super::transaction::*;
use super::trust::IntegrityMode;

#[derive(Debug)]
pub enum ValidationResult {
    Deny,
    Allow,
    #[allow(dead_code)]
    Abstain,
}

pub trait EventValidator: Send + Sync
{
    fn validate(&self, _header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<ValidationResult, ValidationError> {
        Ok(ValidationResult::Abstain)
    }

    fn set_integrity_mode(&mut self, _mode: IntegrityMode) {
    }

    fn clone_validator(&self) -> Box<dyn EventValidator>;

    fn validator_name(&self) -> &str;
}

#[derive(Default, Clone)]
pub struct RubberStampValidator {   
}

impl EventValidator
for RubberStampValidator
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }

    #[allow(unused_variables)]
    fn validate(&self, _header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<ValidationResult, ValidationError>
    {
        Ok(ValidationResult::Allow)
    }

    fn validator_name(&self) -> &str {
        "rubber-stamp-validator"
    }
}

#[derive(Debug, Clone)]
pub struct StaticSignatureValidator {
    #[allow(dead_code)]
    pk: PublicSignKey,
}

impl StaticSignatureValidator
{
    #[allow(dead_code)]
    pub fn new(key: &PublicSignKey) -> StaticSignatureValidator {
        StaticSignatureValidator {
            pk: key.clone(),
        }
    }
}

impl EventValidator
for StaticSignatureValidator
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }
    
    #[allow(unused_variables)]
    fn validate(&self, _header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<ValidationResult, ValidationError>
    {
        Ok(ValidationResult::Allow)
    }

    fn validator_name(&self) -> &str {
        "static-signature-validator"
    }
}

impl Metadata
{
    #[allow(dead_code)]
    pub fn add_signature(&mut self, _sig: MetaSignature) {
    }

    pub fn get_signature<'a>(&'a self) -> Option<&'a MetaSignature> {
        self.core.iter().filter_map(
            |m| {
                match m
                {
                    CoreMetadata::Signature(k) => Some(k),
                     _ => None
                }
            }
        )
        .next()
    }
}