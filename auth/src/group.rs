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
            let mut access = MultiEncryptedSecureData::new(&owner_private_read.as_public_key(), Authorization {
                read: role_read.clone(),
                private_read: role_private_read.clone(),
                write: role_write.clone()
            })?;
            if let RolePurpose::Owner = purpose {
                access.add(&request.sudo_read_key, &owner_private_read)?;
            } else if let RolePurpose::Delegate = purpose {
                access.add(&request.nominal_read_key, &owner_private_read)?;
            } else if let RolePurpose::Observer = purpose {
                access.add(&delegate_private_read.as_public_key(), &owner_private_read)?;
                access.add(&contributor_private_read.as_public_key(), &owner_private_read)?;
            } else {
                access.add(&delegate_private_read.as_public_key(), &owner_private_read)?;
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

    pub async fn process_group_user_add<'a>(&self, request: GroupUserAddRequest, context: InvocationContext<'a>) -> Result<GroupUserAddResponse, ServiceError<GroupUserAddFailed>>
    {
        info!("group ({}) user add", request.group);

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let key_size = request.session.read_keys().map(|k| k.size()).next().unwrap_or_else(|| KeySize::Bit256);
        
        // Compute which chain the group should exist within
        let group_chain_key = auth_chain_key("auth".to_string(), &request.group);
        let chain = context.repository.open_by_key(&group_chain_key).await?;

        // Create the super session that has all the rights we need
        let mut super_session = self.master_session.clone();
        super_session.append(request.session.clone());

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let mut dio = chain.dio(&super_session).await;
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
        let needed_role = match request.purpose {
            RolePurpose::Owner => RolePurpose::Owner,
            RolePurpose::Delegate => RolePurpose::Owner,
            _ => RolePurpose::Delegate
        };

        // Extract the controlling role as this is what we will use to create the role
        let delegate_write = match request.session
            .get_group_role(&request.group, &needed_role)
            .iter()
            .flat_map(|r| r.private_read_keys())
            .next()
        {
            Some(a) => a.clone(),
            None => {
                return Err(ServiceError::Reply(GroupUserAddFailed::NoAccess));
            }
        };

        // If the role does not exist then add it
        if group.roles.iter().any(|r| r.purpose == request.purpose) == false
        {
            // Add this customer role and attach it back to the delegate role
            group.roles.push(Role {
                purpose: request.purpose.clone(),
                access: MultiEncryptedSecureData::new(&delegate_write.as_public_key(), Authorization {
                    read: EncryptKey::generate(key_size),
                    private_read: PrivateEncryptKey::generate(key_size),
                    write: PrivateSignKey::generate(key_size)
                })?
            })
        }

        // Perform the operation that will add the other user to the specific group role
        for role in group.roles.iter_mut().filter(|r| r.purpose == request.purpose) {
            role.access.add(&request.who, &delegate_write)?;
        }

        // Commit
        group.commit(&mut dio)?;
        dio.commit().await?;

        // Return success to the caller
        Ok(GroupUserAddResponse {
            key: group.key().clone(),
        })
    }

    pub async fn process_group_user_remove<'a>(&self, request: GroupUserRemoveRequest, context: InvocationContext<'a>) -> Result<GroupUserRemoveResponse, ServiceError<GroupUserRemoveFailed>>
    {
        info!("group ({}) user remove", request.group);
        
        // Compute which chain the group should exist within
        let group_chain_key = auth_chain_key("auth".to_string(), &request.group);
        let chain = context.repository.open_by_key(&group_chain_key).await?;

        // Create the super session that has all the rights we need
        let mut super_session = self.master_session.clone();
        super_session.append(request.session.clone());

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let mut dio = chain.dio(&super_session).await;
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
        let needed_role = match request.purpose {
            RolePurpose::Owner => RolePurpose::Owner,
            RolePurpose::Delegate => RolePurpose::Owner,
            _ => RolePurpose::Delegate
        };

        // Extract the controlling role as this is what we will use to create the role
        let delegate_write = match request.session
            .get_group_role(&request.group, &needed_role)
            .iter()
            .flat_map(|r| r.private_read_keys())
            .next()
        {
            Some(a) => a.clone(),
            None => {
                return Err(ServiceError::Reply(GroupUserRemoveFailed::NoAccess));
            }
        };
        let delegate_write_hash = delegate_write.as_public_key().hash();

        // Get the group role
        let role = match group.roles.iter_mut().filter(|r| r.purpose == request.purpose).next() {
            Some(a) => a,
            None => {
                return Err(ServiceError::Reply(GroupUserRemoveFailed::RoleNotFound));
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

        // Commit
        group.commit(&mut dio)?;
        dio.commit().await?;

        // Return success to the caller
        Ok(GroupUserRemoveResponse {
            key: group.key().clone(),
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
    let query = crate::query_command(Arc::clone(&registry), username, auth).await?;

    // Extract the read key(s) from the query
    let nominal_read_key = query.advert.nominal_encrypt;
    let sudo_read_key = query.advert.sudo_encrypt;
    
    // Make the create request and fire it over to the authentication server
    let create = CreateGroupRequest {
        group,
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

pub async fn group_user_add_command(group: String, purpose: AteRolePurpose, username: String, auth: Url, session: &AteSession) -> Result<GroupUserAddResponse, GroupUserAddError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_auth(), true).await;
    let chain = Arc::clone(&registry).open_by_url(&chain_url).await?;
    
    // First we query the user that needs to be added so that we can get their public encrypt key
    let query = crate::query_command(Arc::clone(&registry), username, auth).await?;

    // Determine what level of authentication we will associate the role with
    let who = match purpose {
        RolePurpose::Owner => query.advert.sudo_encrypt,
        _ => query.advert.nominal_encrypt
    };
    
    // Make the create request and fire it over to the authentication server
    let create = GroupUserAddRequest {
        group,
        session: session.clone(),
        who,
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

pub async fn group_user_remove_command(group: String, purpose: AteRolePurpose, username: String, auth: Url, session: &AteSession) -> Result<GroupUserRemoveResponse, GroupUserRemoveError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_auth(), true).await;
    let chain = Arc::clone(&registry).open_by_url(&chain_url).await?;
    
    // First we query the user that needs to be added so that we can get their public encrypt key
    let query = crate::query_command(Arc::clone(&registry), username, auth).await?;

    // Determine what level of authentication we will associate the role with
    let who = match purpose {
        RolePurpose::Owner => query.advert.sudo_encrypt,
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
            match RolePurpose::from_str(s.trim()) {
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
            match RolePurpose::from_str(s.trim()) {
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