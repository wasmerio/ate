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
    pub fn get_delegate_write(
        request_session: &AteSessionGroup,
        needed_role: AteRolePurpose,
    ) -> Result<Option<PrivateEncryptKey>, LoadError> {
        let val = {
            request_session
                .get_group_role(&needed_role)
                .iter()
                .flat_map(|r| r.private_read_keys())
                .map(|a| a.clone())
                .next()
        };

        // Extract the controlling role as this is what we will use to create the role
        let delegate_write = match val {
            Some(a) => a,
            None => {
                // If it fails again then give up
                debug!("group-user-add-failed with {}", request_session);
                return Ok(None);
            }
        };

        Ok(Some(delegate_write))
    }

    pub async fn process_group_user_add(
        self: Arc<Self>,
        request: GroupUserAddRequest,
    ) -> Result<GroupUserAddResponse, GroupUserAddFailed> {
        info!("group ({}) user add", request.group);

        // Copy the request session
        let request_purpose = request.purpose;
        let request_session = request.session;

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let key_size = request_session
            .read_keys(AteSessionKeyCategory::AllKeys)
            .map(|k| k.size())
            .next()
            .unwrap_or_else(|| KeySize::Bit192);

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
                return Err(GroupUserAddFailed::GroupNotFound);
            }
            Err(LoadError(
                LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)),
                _,
            )) => {
                return Err(GroupUserAddFailed::NoMasterKey);
            }
            Err(err) => {
                bail!(err);
            }
        };

        // Determine what role is needed to adjust the group
        let needed_role = match &request_purpose {
            AteRolePurpose::Owner => AteRolePurpose::Owner,
            AteRolePurpose::Delegate => AteRolePurpose::Owner,
            _ => AteRolePurpose::Delegate,
        };

        // Get the delegate write key
        let delegate_write = match AuthService::get_delegate_write(&request_session, needed_role)? {
            Some(a) => a,
            None => {
                return Err(GroupUserAddFailed::NoAccess);
            }
        };

        // If the role does not exist then add it
        if group.roles.iter().any(|r| r.purpose == request_purpose) == false {
            // Get our own identity
            let referrer_identity = request_session.inner.identity().to_string();

            // Generate the role keys
            let role_read = EncryptKey::generate(key_size);
            let role_private_read = PrivateEncryptKey::generate(key_size);
            let role_write = PrivateSignKey::generate(key_size);

            // Add this customer role and attach it back to the delegate role
            group.as_mut().roles.push(Role {
                purpose: request_purpose.clone(),
                access: MultiEncryptedSecureData::new(
                    &delegate_write.as_public_key(),
                    referrer_identity,
                    Authorization {
                        read: role_read.clone(),
                        private_read: role_private_read.clone(),
                        write: role_write.clone(),
                    },
                )?,
                read: role_read.hash(),
                private_read: role_private_read.as_public_key().clone(),
                write: role_write.as_public_key().clone(),
            })
        }

        // Perform the operation that will add the other user to the specific group role
        for role in group
            .as_mut()
            .roles
            .iter_mut()
            .filter(|r| r.purpose == request_purpose)
        {
            role.access
                .add(&request.who_key, request.who_name.clone(), &delegate_write)?;
        }

        // Commit
        dio.commit().await?;

        // Return success to the caller
        Ok(GroupUserAddResponse {
            key: group.key().clone(),
        })
    }
}
