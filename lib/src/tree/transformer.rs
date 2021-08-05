#[allow(unused_imports)]
use tracing::{error, info, warn, debug};
use error_chain::bail;
use bytes::Bytes;

use crate::error::*;
use crate::meta::*;
use crate::session::*;
use crate::transform::*;
use crate::transaction::*;

use super::*;

impl EventDataTransformer
for TreeAuthorityPlugin
{
    fn clone_transformer(&self) -> Box<dyn EventDataTransformer> {
        Box::new(self.clone())
    }

    #[allow(unused_variables)]
    fn data_as_underlay(&self, meta: &mut Metadata, with: Bytes, session: &AteSession, trans_meta: &TransactionMetadata) -> Result<Bytes, TransformError>
    {
        let mut with = self.signature_plugin.data_as_underlay(meta, with, session, trans_meta)?;

        let cache = match meta.get_confidentiality() {
            Some(a) => a._cache.as_ref(),
            None => None,
        };

        let auth_store;
        let auth = match &cache {
            Some(a) => a,
            None => {
                auth_store = self.compute_auth(meta, trans_meta, ComputePhase::AfterStore)?;
                &auth_store.read
            }
        };

        if let Some((iv, key)) = self.generate_encrypt_key(auth, session)? {
            let encrypted = key.encrypt_with_iv(&iv, &with[..]);
            meta.core.push(CoreMetadata::InitializationVector(iv));
            with = Bytes::from(encrypted);
        }

        Ok(with)
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &Metadata, with: Bytes, session: &AteSession) -> Result<Bytes, TransformError>
    {
        let mut with = self.signature_plugin.data_as_overlay(meta, with, session)?;

        let iv = meta.get_iv().ok();
        match meta.get_confidentiality() {
            Some(confidentiality) => {
                if let Some(key) = self.get_encrypt_key(meta, confidentiality, iv, session)? {
                    let iv = match iv {
                        Some(a) => a,
                        None => { return Err(TransformError::CryptoError(CryptoError::NoIvPresent)); }
                    };
                    let decrypted = key.decrypt(&iv, &with[..]);
                    with = Bytes::from(decrypted);
                }
            },
            None if iv.is_some() => { return Err(TransformError::UnspecifiedReadability); }
            None => {

            }
        };

        Ok(with)
    }
}