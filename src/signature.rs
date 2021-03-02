use super::index::EventIndexerCore;
use super::validator::EventValidator;
use super::compact::EventCompactor;
use super::lint::EventMetadataLinter;
use super::transform::EventDataTransformer;

use super::plugin::*;
use super::meta::*;

pub struct SignaturePlugin
{
}

impl<M> EventValidator<M>
for SignaturePlugin
where M: OtherMetadata
{
}

impl<M> EventIndexerCore<M>
for SignaturePlugin
where M: OtherMetadata
{
}

impl<M> EventCompactor<M>
for SignaturePlugin
where M: OtherMetadata
{
}

impl<M> EventMetadataLinter<M>
for SignaturePlugin
where M: OtherMetadata
{
    fn metadata_lint(&self, _meta: &mut Metadata<M>) {
    }
}

impl<M> EventDataTransformer<M>
for SignaturePlugin
where M: OtherMetadata
{
}

impl<M> EventPlugin<M>
for SignaturePlugin
where M: OtherMetadata
{
    fn clone_empty(&self) -> Box<dyn EventPlugin<M>> {
        Box::new(
            SignaturePlugin {
            }
        )
    }
}