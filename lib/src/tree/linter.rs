#[allow(unused_imports)]
use tracing::{error, info, warn, debug};
use error_chain::bail;
use std::sync::Arc;

use crate::signature::*;
use crate::error::*;
use crate::meta::*;
use crate::lint::*;
use crate::session::*;
use crate::transaction::*;

use super::*;

impl EventMetadataLinter
for TreeAuthorityPlugin
{
    fn clone_linter(&self) -> Box<dyn EventMetadataLinter> {
        Box::new(self.clone())
    }

    fn metadata_lint_many<'a>(&self, headers: &Vec<LintData<'a>>, session: &AteSession, conversation: Option<&Arc<ConversationSession>>) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();

        let mut other = self.signature_plugin.metadata_lint_many(headers, session, conversation)?;
        ret.append(&mut other);

        Ok(ret)
    }

    fn metadata_lint_event(&self, meta: &Metadata, session: &AteSession, trans_meta: &TransactionMetadata) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();
        let mut sign_with = Vec::new();

        // Signatures a done using the authorizations before its attached
        let auth = self.compute_auth(meta, trans_meta, ComputePhase::BeforeStore)?;
        match auth.write {
            WriteOption::Specific(_) | WriteOption::Any(_) =>
            {
                for write_hash in auth.write.vals().iter()
                {
                    // Add any signing keys that we have
                    sign_with.append(
                        &mut session.write_keys()
                            .filter(|p| p.hash() == *write_hash)
                            .map(|p| p.hash())
                            .collect::<Vec<_>>()
                    );
                }

                if meta.needs_signature() && sign_with.len() <= 0
                {
                    // This record has no authorization
                    return match meta.get_data_key() {
                        Some(key) => Err(LintErrorKind::TrustError(TrustErrorKind::NoAuthorizationWrite(key, auth.write)).into()),
                        None => Err(LintErrorKind::TrustError(TrustErrorKind::NoAuthorizationOrphan).into())
                    };
                }

                // Add the signing key hashes for the later stages
                if sign_with.len() > 0 {
                    ret.push(CoreMetadata::SignWith(MetaSignWith {
                        keys: sign_with,
                    }));
                }
            },
            WriteOption::Inherit => {
                bail!(LintErrorKind::TrustError(TrustErrorKind::UnspecifiedWritability));
            },
            WriteOption::Everyone => { },
            WriteOption::Nobody => { },
        }

        // Now lets add all the encryption keys
        let auth = self.compute_auth(meta, trans_meta, ComputePhase::AfterStore)?;
        let key_hash = match &auth.read {
            ReadOption::Everyone(key) => {
                match key {
                    Some(a) => Some(a.short_hash()),
                    None => None,
                }
            }
            ReadOption::Specific(read_hash, derived) =>
            {
                let mut ret = session.read_keys()
                        .filter(|p| p.hash() == *read_hash)
                        .filter_map(|p| derived.transmute(p).ok())
                        .map(|p| p.short_hash())
                        .next();
                if ret.is_none() {
                    ret = session.private_read_keys()
                        .filter(|p| p.hash() == *read_hash)
                        .filter_map(|p| derived.transmute_private(p).ok())
                        .map(|p| p.short_hash())
                        .next();
                }
                if ret.is_none() {
                    if let Some(key) = meta.get_data_key() {
                        bail!(LintErrorKind::TrustError(TrustErrorKind::NoAuthorizationRead(key, auth.read)));
                    }
                }
                ret
            },
            _ => None,
        };
        if let Some(key_hash) = key_hash {
            ret.push(CoreMetadata::Confidentiality(MetaConfidentiality {
                hash: key_hash,
                _cache: Some(auth.read)
            }));
        }

        // Now run the signature plugin
        ret.extend(self.signature_plugin.metadata_lint_event(meta, session, trans_meta)?);

        // We are done
        Ok(ret)
    }
}