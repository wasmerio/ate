#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;

use ate::prelude::*;
use ate::error::LoadError;
use ate::error::TransformError;
use ate::session::AteRolePurpose;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::model::*;

impl AuthService
{
    pub async fn process_group_user_remove(self: Arc<Self>, request: GroupUserRemoveRequest) -> Result<GroupUserRemoveResponse, ServiceError<GroupUserRemoveFailed>>
    {
        info!("group ({}) user remove", request.group);

        // Copy the request session
        let request_purpose = request.purpose;
        let request_session = request.session;
        
        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key).await?;

        // Create the super session that has all the rights we need
        let mut super_session = self.master_session.clone();
        super_session.append(request_session.clone());

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let dio = chain.dio_full(&super_session).await;
        let mut group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError::NotFound(_)) => {
                return Err(ServiceError::Reply(GroupUserRemoveFailed::GroupNotFound));
            },
            Err(LoadError::TransformationError(TransformError::MissingReadKey(_))) => {
                return Err(ServiceError::Reply(GroupUserRemoveFailed::NoMasterKey));
            },
            Err(err) => {
                return Err(ServiceError::LoadError(err));
            }
        };

        // Determine what role is needed to adjust the group
        let needed_role = match request_purpose {
            AteRolePurpose::Owner => AteRolePurpose::Owner,
            AteRolePurpose::Delegate => AteRolePurpose::Owner,
            _ => AteRolePurpose::Delegate
        };

        // Extract the controlling role as this is what we will use to create the role
        let (delegate_write, _request_session) = match AuthService::get_delegate_write(request_session, group.deref(), needed_role)? {
            Some((a, b)) => (a, b),
            None => {
                return Err(ServiceError::Reply(GroupUserRemoveFailed::NoAccess));
            }
        };
        let delegate_write_hash = delegate_write.as_public_key().hash();

        {
            let mut group = group.as_mut();

            // Get the group role
            let role = {
                match group.roles.iter_mut().filter(|r| r.purpose == request_purpose).next() {
                    Some(a) => a,
                    None => {
                        return Err(ServiceError::Reply(GroupUserRemoveFailed::RoleNotFound));
                    }
                }
            };        

            // Check that we actually have the rights to remove this item
            if role.access.exists(&delegate_write_hash) == false {
                return Err(ServiceError::Reply(GroupUserRemoveFailed::NoAccess));
            }

            // Perform the operation that will remove the other user to the specific group role
            if role.access.remove(&request.who) == false {
                return Err(ServiceError::Reply(GroupUserRemoveFailed::NothingToRemove));
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

pub async fn group_user_remove_command(group: String, purpose: AteRolePurpose, username: String, auth: Url, session: &AteSession) -> Result<GroupUserRemoveResponse, GroupUserRemoveError>
{
    // Open a command chain
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let chain = Arc::clone(&registry).open(&auth, &chain_key_cmd()).await?;
    
    // First we query the user that needs to be added so that we can get their public encrypt key
    let query = crate::query_command(Arc::clone(&registry), username, auth).await?;

    // Determine what level of authentication we will associate the role with
    let who = match purpose {
        AteRolePurpose::Owner => query.advert.sudo_encrypt,
        _ => query.advert.nominal_encrypt
    };
    
    // Make the create request and fire it over to the authentication server
    let create = GroupUserRemoveRequest {
        group,
        session: session.clone(),
        who: who.hash(),
        purpose,
    };

    let response: Result<GroupUserRemoveResponse, InvokeError<GroupUserRemoveFailed>> = chain.invoke(create).await;
    match response {
        Err(InvokeError::Reply(GroupUserRemoveFailed::NoMasterKey)) => Err(GroupUserRemoveError::NoMasterKey),
        Err(InvokeError::Reply(GroupUserRemoveFailed::NoAccess)) => Err(GroupUserRemoveError::NoAccess),
        Err(InvokeError::Reply(GroupUserRemoveFailed::GroupNotFound)) => Err(GroupUserRemoveError::GroupNotFound),
        Err(InvokeError::Reply(GroupUserRemoveFailed::RoleNotFound)) => Err(GroupUserRemoveError::RoleNotFound),
        Err(InvokeError::Reply(GroupUserRemoveFailed::NothingToRemove)) => Err(GroupUserRemoveError::NothingToRemove),
        result => {
            let result = result?;
            debug!("key: {}", result.key);
            Ok(result)
        }
    }
}

pub async fn main_group_user_remove(
    group: Option<String>,
    purpose: Option<AteRolePurpose>,
    username: Option<String>,
    auth: Url,
    session: &AteSession
) -> Result<(), GroupUserRemoveError>
{
    let group = match group {
        Some(a) => a,
        None => {
            print!("Group: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid group");
            s.trim().to_string()
        }
    };

    let purpose = match purpose {
        Some(a) => a,
        None => {
            print!("Role: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid role purpose");
            match AteRolePurpose::from_str(s.trim()) {
                Ok(a) => a,
                Err(err) => { return Err(GroupUserRemoveError::InvalidPurpose(err.to_string())); }
            }
        }
    };

    let username = match username {
        Some(a) => a,
        None => {
            print!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    // Remove a user from a group using the authentication server
    let result = group_user_remove_command(group, purpose, username, auth, session).await?;

    println!("Group user removed (id={})", result.key);

    Ok(())
}