#[allow(unused_imports)]
use tracing::{error, info, warn, debug};

use crate::crypto::*;
use crate::error::*;
use crate::meta::*;
use crate::session::*;

use super::*;

impl TreeAuthorityPlugin
{
    pub(super) fn generate_encrypt_key(&self, auth: &ReadOption, session: &AteSession) -> Result<Option<(InitializationVector, EncryptKey)>, TransformError>
    {
        match auth {
            ReadOption::Inherit => {
                Err(TransformErrorKind::UnspecifiedReadability.into())
            },
            ReadOption::Everyone(_key) => {
                Ok(None)
            },
            ReadOption::Specific(key_hash, derived) => {
                for key in session.read_keys() {
                    if key.hash() == *key_hash {
                        return Ok(Some((
                            InitializationVector::generate(),
                            derived.transmute(key)?
                        )));
                    }
                }
                for key in session.private_read_keys() {
                    if key.hash() == *key_hash {
                        return Ok(Some((
                            InitializationVector::generate(),
                            derived.transmute_private(key)?
                        )));
                    }
                }
                Err(TransformErrorKind::MissingReadKey(key_hash.clone()).into())
            }
        }
    }
}