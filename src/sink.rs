use super::meta::OtherMetadata;
use super::meta::Metadata;

pub trait EventSink<M>
where M: OtherMetadata
{
    fn feed(&mut self, _meta: &Metadata<M>) {
    }
}