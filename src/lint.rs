use super::meta::*;
#[allow(unused_imports)]
use openssl::symm::{encrypt, Cipher};

pub trait EventMetadataLinter<M>
where M: OtherMetadata
{
    /// Called just before the metadata is pushed into the redo log
    fn metadata_lint(&self, _meta: &mut Metadata<M>) {
    }

    /// Callback when metadata is used by an actual user
    fn metadata_trim(&self, _meta: &mut Metadata<M>) {
    }
}