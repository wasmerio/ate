#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;
use regex::Regex;

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
    pub async fn process_create_group(self: Arc<Self>, request: CreateGroupRequest) -> Result<CreateGroupResponse, CreateGroupFailed>
    {
        info!("create group: {}", request.group);

        // Make sure the group matches the regex and is valid
        let regex = Regex::new("^/{0,1}([a-zA-Z0-9_]{0,})$").unwrap();
        if let Some(_captures) = regex.captures(request.group.as_str()) {
            if request.group.len() <= 0 {
                return Err(CreateGroupFailed::InvalidGroupName);
            }
        } else {
            return Err(CreateGroupFailed::InvalidGroupName);
        }

        // Get the master write key
        let master_write_key = match self.master_session.write_keys().next() {
            Some(a) => a.clone(),
            None => {
                return Err(CreateGroupFailed::NoMasterKey);
            }
        };

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let key_size = request.nominal_read_key.size();
        let master_key = match self.master_key() {
            Some(a) => a,
            None => { return Err(CreateGroupFailed::NoMasterKey); }
        };
        
        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key).await?;
        let dio = chain.dio_mut(&self.master_session).await;

        // Try and find a free GID
        let mut gid = None;
        for n in 0u32..50u32 {
            let gid_test = estimate_group_name_as_gid(request.group.clone()) + n;
            if gid_test < 1000 { continue; }
            if dio.exists(&PrimaryKey::from(gid_test as u64)).await {
                continue;
            }
            gid = Some(gid_test);
            break;
        }
        let gid = match gid {
            Some(a) => a,
            None => {
                return Err(CreateGroupFailed::NoMoreRoom);
            }
        };
        
        // If it already exists then fail
        let group_key = PrimaryKey::from(request.group.clone());
        if dio.exists(&group_key).await {
            return Err(CreateGroupFailed::AlreadyExists);
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

        // Generate the observer encryption keys used to protect this role
        let observer_read = EncryptKey::generate(key_size);
        let observer_private_read = PrivateEncryptKey::generate(key_size);
        let observer_write = PrivateSignKey::generate(key_size);

        // The super session needs the owner keys so that it can save the records
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&owner_read);
        super_session.user.add_read_key(&delegate_read);
        super_session.user.add_private_read_key(&owner_private_read);
        super_session.user.add_private_read_key(&delegate_private_read);
        super_session.user.add_write_key(&owner_write);
        super_session.user.add_write_key(&delegate_write);
        let dio = chain.dio_full(&super_session).await;
        
        // Create the group and save it
        let group = Group {
            name: request.group.clone(),
            foreign: DaoForeign::default(),
            gid,
            roles: Vec::new(),
        };
        let mut group = dio.store_with_key(group, group_key.clone())?;

        // Create the session that we will return to the call
        let mut session = AteSession::default();

        // Add the other roles
        {
            let mut group_mut = group.as_mut();
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
                    AteRolePurpose::Owner => {
                        role_read = owner_read.clone();
                        role_private_read = owner_private_read.clone();
                        role_write = owner_write.clone();
                    },
                    AteRolePurpose::Delegate => {
                        role_read = delegate_read.clone();
                        role_private_read = delegate_private_read.clone();
                        role_write = delegate_write.clone();
                    },
                    AteRolePurpose::Contributor => {
                        role_read = contributor_read.clone();
                        role_private_read = contributor_private_read.clone();
                        role_write = contributor_write.clone();
                    },
                    AteRolePurpose::Observer => {
                        role_read = observer_read.clone();
                        role_private_read = observer_private_read.clone();
                        role_write = observer_write.clone();
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
                if let AteRolePurpose::Owner = purpose {
                    access.add(&request.sudo_read_key, request.identity.clone(), &owner_private_read)?;
                } else if let AteRolePurpose::Delegate = purpose {
                    access.add(&request.nominal_read_key, request.identity.clone(), &owner_private_read)?;
                } else if let AteRolePurpose::Observer = purpose {
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
                    read: role_read.hash(),
                    private_read: role_private_read.as_public_key(),
                    write: role_write.as_public_key(),
                    access,
                };
                group_mut.roles.push(role);
            }
        }

        // Set all the permissions and save the group. While the group is readable by everyone
        // the data held within the structure is itself encrypted using the MultiEncryptedSecureData
        // object which allows one to multiplex the access to the keys
        group.auth_mut().read = ReadOption::from_key(&master_key);
        group.auth_mut().write = WriteOption::Any(vec![master_write_key.hash(), owner_write.hash()]);

        // Commit
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
    let registry = ate::mesh::Registry::new( &conf_cmd()).await.cement();
    let chain = Arc::clone(&registry).open(&auth, &chain_key_cmd()).await?;

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

    let response: Result<CreateGroupResponse, CreateGroupFailed> = chain.invoke(create).await?;
    let result = response?;
    debug!("key: {}", result.key);
    Ok(result)
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