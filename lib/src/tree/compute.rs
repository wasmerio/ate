#[allow(unused_imports)]
use log::{error, info, warn, debug};

use crate::error::*;
use crate::meta::*;
use crate::transaction::*;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ComputePhase
{
    BeforeStore,
    AfterStore,
}

impl TreeAuthorityPlugin
{
    pub(super) fn compute_auth(&self, meta: &Metadata, trans_meta: &TransactionMetadata, phase: ComputePhase) -> Result<MetaAuthorization, TrustError>
    {
        // If its not got a key then it just inherits the permissions of the root
        let key = match meta.get_data_key() {
            Some(a) => a,
            None => {
                return Ok(
                    MetaAuthorization {
                        read: ReadOption::Everyone(None),
                        write: self.root.clone(),
                    }
                );
            }
        };
        #[cfg(feature = "super_verbose")]
        debug!("compute_auth(): key={}", key);

        // Get the authorization of this node itself (if its post phase)
        let mut auth = match phase {
            ComputePhase::BeforeStore => None,
            ComputePhase::AfterStore => meta.get_authorization(),
        };
        
        // In the scenarios that this is before the record is saved or
        // if no authorization is attached to the record then we fall
        // back to whatever is the value in the existing chain of trust
        if auth.is_none() {
            auth = trans_meta.auth.get(&key);
            if auth.is_none() {
                auth = self.auth.get(&key);
            }
        }

        // Fall back on inheriting from the parent if there is no
        // record yet set for this data object
        let (mut read, mut write) = match auth {
            Some(a) => (a.read.clone(), a.write.clone()),
            None => (ReadOption::Inherit, WriteOption::Inherit),
        };
        #[cfg(feature = "super_verbose")]
        debug!("compute_auth(): read={}, write={}", read, write);

        // Resolve any inheritance through recursive queries
        let mut parent = meta.get_parent();
        while (read == ReadOption::Inherit || write == WriteOption::Inherit)
               && parent.is_some()
        {
            {
                let parent = match parent {
                    Some(a) => a.vec.parent_id,
                    None => unreachable!(),
                };
                #[cfg(feature = "super_verbose")]
                debug!("compute_auth(): parent={}", parent);

                // Get the authorization for this parent (if there is one)
                let mut parent_auth = trans_meta.auth.get(&parent);
                if parent_auth.is_none() {
                    parent_auth = self.auth.get(&parent);
                }
                let parent_auth = match parent_auth {
                    Some(a) => a,
                    None => {
                        #[cfg(feature = "super_verbose")]
                        debug!("compute_auth(): missing_parent={}", parent);
                        return Err(TrustError::MissingParent(parent));
                    }
                };

                // Resolve the read inheritance
                if read == ReadOption::Inherit {
                    read = parent_auth.read.clone();
                }
                // Resolve the write inheritance
                if write == WriteOption::Inherit {
                    write = parent_auth.write.clone();
                }

                #[cfg(feature = "super_verbose")]
                debug!("compute_auth(): read={}, write={}", read, write);
            }

            // Walk up the tree until we have a resolved inheritance or there are no more parents
            parent = match parent {
                Some(a) => {
                    let mut r = trans_meta.parents.get(&a.vec.parent_id);
                    if r.is_none() {
                        r = match self.parents.get(&a.vec.parent_id) {
                            Some(b) if b.vec.parent_id != a.vec.parent_id => Some(b),
                            _ => None,
                        };
                    }
                    match r {
                        Some(a) => Some(a),
                        None => { break; }
                    }
                },
                None => unreachable!(),
            }
        }

        // If we are at the top of the walk and its still inherit then we inherit the
        // permissions of a root node
        if read == ReadOption::Inherit {
            read = ReadOption::Everyone(None);
        }
        if write == WriteOption::Inherit {
            #[cfg(feature = "super_verbose")]
            debug!("compute_auth(): using_root_read={}", self.root);
            write = self.root.clone();
        }
        #[cfg(feature = "super_verbose")]
        debug!("compute_auth(): read={}, write={}", read, write);

        let auth = MetaAuthorization {
            read,
            write,
        };

        // Return the result
        Ok(auth)
    }
}