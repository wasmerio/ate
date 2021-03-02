use super::meta::*;
use tokio::io::Result;

#[derive(Debug)]
pub enum ValidationResult {
    Allow,
    #[allow(dead_code)]
    Abstain,
    #[allow(dead_code)]
    Deny,
}

pub trait EventValidator<M>
where M: OtherMetadata
{
    fn validate(&self, _evt: &Header<M>) -> Result<ValidationResult> {
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
    fn validate(&self, _evt: &Header<M>) -> Result<ValidationResult>
    {
        Ok(ValidationResult::Allow)
    }
}