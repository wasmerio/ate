use super::meta::Metadata;
use super::crypto::Hash;
use super::error::*;

pub trait EventSink
{
    fn feed(&mut self, _meta: &Metadata, _data_hash: &Option<Hash>) -> Result<(), SinkError> {
        Ok(())
    }
}