use super::meta::OtherMetadata;
use super::meta::MetadataExt;
use super::crypto::Hash;
use super::error::*;

pub trait EventSink<M>
where M: OtherMetadata
{
    fn feed(&mut self, _data_hash: &Option<Hash>, _meta: &MetadataExt<M>) -> Result<(), SinkError> {
        Ok(())
    }
}