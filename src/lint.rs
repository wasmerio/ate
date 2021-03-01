use super::header::*;
use tokio::io::Result;
#[allow(unused_imports)]
use openssl::symm::{encrypt, Cipher};

pub trait EventMetadataLinter<M>
where M: OtherMetadata
{
    /// Called just before the metadata is pushed into the redo log
    fn metadata_lint(&self, meta: &mut Metadata<M>) -> Result<Metadata<M>>;

    /// Callback when metadata is used by an actual user
    fn metadata_trim(&self, meta: &mut Metadata<M>) -> Result<Metadata<M>>;
}