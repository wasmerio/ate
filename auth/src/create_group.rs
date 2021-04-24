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
use ate::session::RolePurpose;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::model::*;

impl AuthService
{
    pub async fn process_create_group<'a>(&self, request: CreateGroupRequest, context: InvocationContext<'a>) -> Result<CreateGroupResponse, ServiceError<CreateGroupFailed>>
    {
        info!("create group: {}", request.group);

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let key_size = request.nominal_read_key.size();
        let master_key = match self.master_key() {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateGroupFailed::NoMasterKey)); }
        };
        
        // Compute which chain the group should exist within
        let group_chain_key = auth_chain_key("auth".to_string(), &request.group);
        let chain = context.repository.open_by_key(&group_chain_key).await?;
        
        // If it already exists then fail
        let group_key = PrimaryKey::from(request.group.clone());
        let mut dio = chain.dio(&self.master_session).await;
        if dio.exists(&group_key).await {
            return Err(ServiceError::Reply(CreateGroupFailed::AlreadyExists));
        }

        // Generate the owner encryption keys used to protect this role
        let owner_read = EncryptKey::generate(key_size);
        let owner_private_read = PrivateEncryptKey::generate(key_size);
        let owner_write = PrivateSignKey::generate(key_size);

        // Generate the delegate encryption keys used to protect this role
        let delegate_read = EncryptKey::generate(key_size);
        let delegate_private_read = PrivateEncryptKey::generate(key_size);
        let delegate_write = PrivateSignKey::generate(key_size);

        // Generate the contributor encryption keys used to protect this role
        let contributor_read = EncryptKey::generate(key_size);
        let contributor_private_read = PrivateEncryptKey::generate(key_size);
        let contributor_write = PrivateSignKey::generate(key_size);

        // The super session needs the owner keys so that it can save the records
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&owner_read);
        super_session.user.add_read_key(&delegate_read);
        super_session.user.add_private_read_key(&owner_private_read);
        super_session.user.add_private_read_key(&delegate_private_read);
        super_session.user.add_write_key(&owner_write);
        super_session.user.add_write_key(&delegate_write);
        let mut dio = chain.dio(&super_session).await;
        
        // Create the group and save it
        let group = Group {
            name: request.group.clone(),
            roles: Vec::new(),
        };
        let mut group = Dao::make(group_key.clone(), chain.default_format(), group);

        // Create the session that we will return to the call
        let mut session = AteSession::default();

        // Add the other roles
        for purpose in vec![
            AteRolePurpose::Owner,
            AteRolePurpose::Delegate,
            AteRolePurpose::Contributor,
            AteRolePurpose::Observer
        ].iter()
        {
            // Generate the keys
            let role_read;
            let role_private_read;
            let role_write;
            match purpose {
                RolePurpose::Owner => {
                    role_read = owner_read.clone();
                    role_private_read = owner_private_read.clone();
                    role_write = owner_write.clone();
                },
                RolePurpose::Delegate => {
                    role_read = delegate_read.clone();
                    role_private_read = delegate_private_read.clone();
                    role_write = delegate_write.clone();
                },
                RolePurpose::Contributor => {
                    role_read = contributor_read.clone();
                    role_private_read = contributor_private_read.clone();
                    role_write = contributor_write.clone();
                },
                _ => {
                    role_read = EncryptKey::generate(key_size);
                    role_private_read = PrivateEncryptKey::generate(key_size);
                    role_write = PrivateSignKey::generate(key_size);
                }
            }

            // Create the access object
            let mut access = MultiEncryptedSecureData::new(&owner_private_read.as_public_key(), "owner".to_string(), Authorization {
                read: role_read.clone(),
                private_read: role_private_read.clone(),
                write: role_write.clone()
            })?;
            if let RolePurpose::Owner = purpose {
                access.add(&request.sudo_read_key, request.identity.clone(), &owner_private_read)?;
            } else if let RolePurpose::Delegate = purpose {
                access.add(&request.nominal_read_key, request.identity.clone(), &owner_private_read)?;
            } else if let RolePurpose::Observer = purpose {
                access.add(&delegate_private_read.as_public_key(), "delegate".to_string(), &owner_private_read)?;
                access.add(&contributor_private_read.as_public_key(), "contributor".to_string(), &owner_private_read)?;
            } else {
                access.add(&delegate_private_read.as_public_key(), "delegate".to_string(), &owner_private_read)?;
            }

            // Add the rights to the session we will return
            let role = session.get_or_create_group_role(&request.group, &purpose);
            role.add_read_key(&role_read.clone());
            role.add_private_read_key(&role_private_read.clone());
            role.add_write_key(&role_write.clone());

            // Add the owner role to the group (as its a super_key the authentication server
            // is required to read the group records and load them, while the authentication
            // server can run in a distributed mode it is a centralized authority)
            let role = Role {
                purpose: purpose.clone(),
                access,
            };
            group.roles.push(role);
        }

        // Set all the permissions and save the group. While the group is readable by everyone
        // the data held within the structure is itself encrypted using the MultiEncryptedSecureData
        // object which allows one to multiplex the access to the keys
        group.auth_mut().read = ReadOption::Specific(master_key.hash());
        group.auth_mut().write = WriteOption::Inherit;

        // Commit
        let group = group.commit(&mut dio)?;
        dio.commit().await?;

        // Add the group credentials to the response
        let session = complete_group_auth(group.deref(), session)?;

        // Return success to the caller
        Ok(CreateGroupResponse {
            key: group.key().clone(),
            session,
        })
    }
}

pub async fn create_group_command(group: String, auth: Url, username: String) -> Result<CreateGroupResponse, CreateError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_auth(), true).await;
    let chain = Arc::clone(&registry).open_by_url(&chain_url).await?;

    // First we query the user that needs to be added so that we can get their public encrypt key
    let query = crate::query_command(Arc::clone(&registry), username.clone(), auth).await?;

    // Extract the read key(s) from the query
    let nominal_read_key = query.advert.nominal_encrypt;
    let sudo_read_key = query.advert.sudo_encrypt;
    
    // Make the create request and fire it over to the authentication server
    let create = CreateGroupRequest {
        group,
        identity: username.clone(),
        nominal_read_key,
        sudo_read_key,
    };

    let response: Result<CreateGroupResponse, InvokeError<CreateGroupFailed>> = chain.invoke(create).await;
    match response {
        Err(InvokeError::Reply(CreateGroupFailed::AlreadyExists)) => Err(CreateError::AlreadyExists),
        result => {
            let result = result?;
            debug!("key: {}", result.key);
            Ok(result)
        }
    }
}

pub async fn main_create_group(
    group: Option<String>,
    auth: Url,
    username: Option<String>
) -> Result<AteSession, CreateError>
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

    // Create a user using the authentication server which will give us a session with all the tokens
    let result = create_group_command(group, auth, username).await?;
    println!("Group created (id={})", result.key);

    Ok(result.session)
}