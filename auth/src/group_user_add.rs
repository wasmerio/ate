#![allow(unused_imports)]
use log::{info, error, debug};
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
    pub fn get_delegate_write(mut request_session: AteSession, group: &Group, needed_role: AteRolePurpose) -> Result<Option<(PrivateEncryptKey, AteSession)>, LoadError>
    {
        let group_name = group.name.clone();
        let mut delegate_check_first_time = true;
        let delegate_write;
        loop {
            let val = {
                request_session
                    .get_group_role(&group_name, &needed_role)
                    .iter()
                    .flat_map(|r| r.private_read_keys())
                    .map(|a| a.clone())
                    .next()
            };

            // Extract the controlling role as this is what we will use to create the role
            delegate_write = match val
            {
                Some(a) => a,
                None =>
                {
                    if delegate_check_first_time {
                        delegate_check_first_time = false;

                        // Attempt to get the access via the gather call
                        request_session = complete_group_auth(group.deref(), request_session)?;
                        continue;
                    }

                    // If it fails again then give up
                    debug!("group-user-add-failed with {}", request_session);
                    return Ok(None);
                }
            };
            break;
        }

        Ok(Some((delegate_write, request_session)))
    }

    pub async fn process_group_user_add<'a>(&self, request: GroupUserAddRequest, context: InvocationContext<'a>) -> Result<GroupUserAddResponse, ServiceError<GroupUserAddFailed>>
    {
        info!("group ({}) user add", request.group);

        // Copy the request session
        let request_purpose = request.purpose;
        let request_session = request.session;

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let key_size = request_session.read_keys().map(|k| k.size()).next().unwrap_or_else(|| KeySize::Bit256);

        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = context.repository.open(&self.auth_url, &group_chain_key).await?;

        // Create the super session that has all the rights we need
        let mut super_session = self.master_session.clone();
        super_session.append(request_session.clone());

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let dio = chain.dio_full(&super_session).await;
        let mut group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError::NotFound(_)) => {
                return Err(ServiceError::Reply(GroupUserAddFailed::GroupNotFound));
            },
            Err(LoadError::TransformationError(TransformError::MissingReadKey(_))) => {
                return Err(ServiceError::Reply(GroupUserAddFailed::NoMasterKey));
            },
            Err(err) => {
                return Err(ServiceError::LoadError(err));
            }
        };

        // Determine what role is needed to adjust the group
        let needed_role = match &request_purpose {
            AteRolePurpose::Owner => AteRolePurpose::Owner,
            AteRolePurpose::Delegate => AteRolePurpose::Owner,
            _ => AteRolePurpose::Delegate
        };

        // Get the delegate write key
        let (delegate_write, request_session) = match AuthService::get_delegate_write(request_session, group.deref(), needed_role)? {
            Some((a, b)) => (a, b),
            None => {
                return Err(ServiceError::Reply(GroupUserAddFailed::NoAccess));
            }
        };

        // If the role does not exist then add it
        if group.roles.iter().any(|r| r.purpose == request_purpose) == false
        {
            // Get our own identity
            let referrer_identity = match request_session.user.identity() {
                Some(a) => a.clone(),
                None => {
                    return Err(ServiceError::Reply(GroupUserAddFailed::UnknownIdentity));
                }
            };

            // Generate the role keys
            let role_read = EncryptKey::generate(key_size);
            let role_private_read = PrivateEncryptKey::generate(key_size);
            let role_write = PrivateSignKey::generate(key_size);

            // Add this customer role and attach it back to the delegate role
            group.as_mut().roles.push(Role {
                purpose: request_purpose.clone(),
                access: MultiEncryptedSecureData::new(&delegate_write.as_public_key(), referrer_identity, Authorization {
                    read: role_read.clone(),
                    private_read: role_private_read.clone(),
                    write: role_write.clone()
                })?,
                read: role_read.hash(),
                private_read: role_private_read.as_public_key(),
                write: role_write.as_public_key(),
            })
        }

        // Perform the operation that will add the other user to the specific group role
        for role in group.as_mut().roles.iter_mut().filter(|r| r.purpose == request_purpose) {
            role.access.add(&request.who_key, request.who_name.clone(), &delegate_write)?;
        }

        // Commit
        dio.commit().await?;

        // Return success to the caller
        Ok(GroupUserAddResponse {
            key: group.key().clone(),
        })
    }
}

pub async fn group_user_add_command(group: String, purpose: AteRolePurpose, username: String, auth: Url, session: &AteSession) -> Result<GroupUserAddResponse, GroupUserAddError>
{
    // Open a command chain
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let chain = Arc::clone(&registry).open(&auth, &chain_key_cmd()).await?;
    
    // First we query the user that needs to be added so that we can get their public encrypt key
    let query = crate::query_command(Arc::clone(&registry), username.clone(), auth).await?;

    // Determine what level of authentication we will associate the role with
    let who_key = match purpose {
        AteRolePurpose::Owner => query.advert.sudo_encrypt,
        _ => query.advert.nominal_encrypt
    };
    
    // Make the create request and fire it over to the authentication server
    let create = GroupUserAddRequest {
        group,
        session: session.clone(),
        who_name: username.clone(),
        who_key,
        purpose,
    };

    let response: Result<GroupUserAddResponse, InvokeError<GroupUserAddFailed>> = chain.invoke(create).await;
    match response {
        Err(InvokeError::Reply(GroupUserAddFailed::NoMasterKey)) => Err(GroupUserAddError::NoMasterKey),
        Err(InvokeError::Reply(GroupUserAddFailed::NoAccess)) => Err(GroupUserAddError::NoAccess),
        result => {
            let result = result?;
            debug!("key: {}", result.key);
            Ok(result)
        }
    }
}

pub async fn main_group_user_add(
    group: Option<String>,
    purpose: Option<AteRolePurpose>,
    username: Option<String>,
    auth: Url,
    session: &AteSession
) -> Result<(), GroupUserAddError>
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
                Err(err) => { return Err(GroupUserAddError::InvalidPurpose(err.to_string())); }
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

    // Add a user in a group using the authentication server
    let result = group_user_add_command(group, purpose, username, auth, session).await?;

    println!("Group user added (id={})", result.key);

    Ok(())
}