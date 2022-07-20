#![allow(unused_imports)]
use error_chain::bail;
use qrcode::render::unicode;
use qrcode::QrCode;
use std::io::stdout;
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use ate::error::LoadError;
use ate::error::TransformError;
use ate::prelude::*;
use ate::session::AteRolePurpose;
use ate::utils::chain_key_4hex;

use crate::error::*;
use crate::helper::*;
use crate::model::*;
use crate::prelude::*;
use crate::request::*;
use crate::service::AuthService;

impl AuthService {
    pub async fn process_group_user_remove(
        self: Arc<Self>,
        request: GroupUserRemoveRequest,
    ) -> Result<GroupUserRemoveResponse, GroupUserRemoveFailed> {
        info!("group ({}) user remove", request.group);

        // Copy the request session
        let request_purpose = request.purpose;
        let request_session = request.session;

        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key, true).await?;

        // Create the super session that has all the rights we need
        let mut super_session = self.master_session.clone();
        super_session.append(request_session.properties());

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let dio = chain.dio_full(&super_session).await;
        let mut group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                return Err(GroupUserRemoveFailed::GroupNotFound);
            }
            Err(LoadError(
                LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)),
                _,
            )) => {
                return Err(GroupUserRemoveFailed::NoMasterKey);
            }
            Err(err) => {
                bail!(err);
            }
        };

        // Determine what role is needed to adjust the group
        let needed_role = match request_purpose {
            AteRolePurpose::Owner => AteRolePurpose::Owner,
            AteRolePurpose::Delegate => AteRolePurpose::Owner,
            _ => AteRolePurpose::Delegate,
        };

        // Extract the controlling role as this is what we will use to create the role
        let delegate_write = match AuthService::get_delegate_write(&request_session, needed_role)? {
            Some(a) => a,
            None => {
                return Err(GroupUserRemoveFailed::NoAccess);
            }
        };
        let delegate_write_hash = delegate_write.as_public_key().hash();

        {
            let mut group = group.as_mut();

            // Get the group role
            let role = {
                match group
                    .roles
                    .iter_mut()
                    .filter(|r| r.purpose == request_purpose)
                    .next()
                {
                    Some(a) => a,
                    None => {
                        return Err(GroupUserRemoveFailed::RoleNotFound);
                    }
                }
            };

            // Check that we actually have the rights to remove this item
            if role.access.exists(&delegate_write_hash) == false {
                return Err(GroupUserRemoveFailed::NoAccess);
            }

            // Perform the operation that will remove the other user to the specific group role
            if role.access.remove(&request.who) == false {
                return Err(GroupUserRemoveFailed::NothingToRemove);
            }
        }

        // Commit
        dio.commit().await?;

        // Return success to the caller
        Ok(GroupUserRemoveResponse {
            key: group.key().clone(),
        })
    }
}
