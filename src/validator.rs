use super::event::*;
use super::header::*;
use tokio::io::Result;

pub trait EventValidator<M>
    where M: MetadataTrait
{
    fn validate(&self, evt: &Event<M>) -> Result<()>;
}

#[derive(Default)]
pub struct RubberStampValidator
{   
}

impl<M> EventValidator<M> for RubberStampValidator
    where M: MetadataTrait
{
    #[allow(unused_variables)]
    fn validate(&self, evt: &Event<M>) -> Result<()>
    {
        Ok(())
    }
}