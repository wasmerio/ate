use super::meta::MetadataExt;
use super::crypto::Hash;
use super::meta::OtherMetadata;
use super::error::*;

pub trait EventSink<M>
where M: OtherMetadata
{
    fn feed(&mut self, _meta: &MetadataExt<M>, _data_hash: &Option<Hash>) -> Result<(), SinkError> {
        Ok(())
    }
}