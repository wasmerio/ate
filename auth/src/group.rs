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
        info!("create group: {}", request.name);

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let key_size = request.read_key.size();
        let master_key = match self.master_key() {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateGroupFailed::NoMasterKey)); }
        };
        
        // Compute which chain the group should exist within
        let group_chain_key = auth_chain_key("auth".to_string(), &request.name);
        let chain = context.repository.open_by_key(&group_chain_key).await?;
        
        // If it already exists then fail
        let group_key = PrimaryKey::from(request.name.clone());
        let mut dio = chain.dio(&self.master_session).await;
        if dio.exists(&group_key).await {
            return Err(ServiceError::Reply(CreateGroupFailed::AlreadyExists));
        }

        // Generate the owner encryption keys used to protect this role
        let owner_read = EncryptKey::generate(key_size);
        let owner_private_read = PrivateEncryptKey::generate(key_size);
        let owner_write = PrivateSignKey::generate(key_size);

        // The super session needs the owner keys so that it can save the records
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&owner_read);
        super_session.user.add_write_key(&owner_write);
        let mut dio = chain.dio(&super_session).await;
        
        // Create the group and save it
        let group = Group {
            name: request.name.clone(),
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
                access.add(&request.read_key, &owner_private_read)?;
            }

            // Add the rights to the session we will return
            let role = session.get_or_create_group_role(Some(request.name.clone()), purpose.clone());
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
        group.auth_mut().write = WriteOption::Specific(owner_write.hash());

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

    pub async fn process_group_add<'a>(&self, request: GroupAddRequest, context: InvocationContext<'a>) -> Result<GroupAddResponse, ServiceError<GroupAddFailed>>
    {
        info!("group add: {}", request.group);

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let key_size = request.referrer.size();
        
        // Compute which chain the group should exist within
        let group_chain_key = auth_chain_key("auth".to_string(), &request.group);
        let chain = context.repository.open_by_key(&group_chain_key).await?;

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let mut dio = chain.dio(&self.master_session).await;
        let mut group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError::NotFound(_)) => {
                return Err(ServiceError::Reply(GroupAddFailed::GroupNotFound));
            },
            Err(LoadError::TransformationError(TransformError::MissingReadKey(_))) => {
                return Err(ServiceError::Reply(GroupAddFailed::NoMasterKey));
            },
            Err(err) => {
                return Err(ServiceError::LoadError(err));
            }
        };

        // Create the super session by gaining more permissions then restart the DIO
        let mut super_session = self.master_session.clone();
        super_session.user.add_private_read_key(&request.referrer);
        let super_session = complete_group_auth(group.deref(), super_session)?;
        let mut dio = chain.dio(&super_session).await;

        // If the role does not exist then add it
        if group.roles.iter().any(|r| r.purpose == request.purpose) == false {
            group.roles.push(Role {
                purpose: request.purpose.clone(),
                access: MultiEncryptedSecureData::new(&request.referrer.as_public_key(), Authorization {
                    read: EncryptKey::generate(key_size),
                    private_read: PrivateEncryptKey::generate(key_size),
                    write: PrivateSignKey::generate(key_size)
                })?
            })
        }

        // Perform the operation that will add the other user to the specific group role
        for role in group.roles.iter_mut().filter(|r| r.purpose == request.purpose) {
            role.access.add(&request.who, &request.referrer)?;
        }

        // Commit
        group.commit(&mut dio)?;
        dio.commit().await?;

        // Return success to the caller
        Ok(GroupAddResponse {
            key: group.key().clone(),
        })
    }
}

pub async fn create_group_command(name: String, auth: Url, session: &AteSession) -> Result<CreateGroupResponse, CreateError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_auth(), true).await;
    let chain = registry.open_by_url(&chain_url).await?;

    // Extract the read key from the session that will be used for the owner
    // key on the file-system
    let read_key = match session.user.private_read_keys().next() {
        Some(a) => a.as_public_key(),
        None => { return Err(CreateError::MissingReadKey); }
    };
    
    // Make the create request and fire it over to the authentication server
    let create = CreateGroupRequest {
        name,
        read_key,
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
    name: Option<String>,
    auth: Url,
    session: &AteSession
) -> Result<AteSession, CreateError>
{
    let name = match name {
        Some(a) => a,
        None => {
            print!("Group: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid name");
            s.trim().to_string()
        }
    };

    // Create a user using the authentication server which will give us a session with all the tokens
    let result = create_group_command(name, auth, session).await?;
    println!("Group created (id={})", result.key);

    // Create the session
    let mut session = session.clone();
    session.append(result.session);
    Ok(session)
}

pub async fn group_add_command(group: String, purpose: AteRolePurpose, username: String, auth: Url, session: &AteSession) -> Result<GroupAddResponse, GroupAddError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_auth(), true).await;
    let chain = Arc::clone(&registry).open_by_url(&chain_url).await?;

    // Extract the read key from the session that will be used for the owner
    // key on the file-system
    let read_key = match session.get_group_role(Some(group.clone()), RolePurpose::Owner).iter().flat_map(|r| r.private_read_keys()).next() {
        Some(a) => a.clone(),
        None => {
            debug!("Session is missing private read key for group ({}) with Ownership role", group);
            return Err(GroupAddError::NoAccess);
        }
    };
    
    // First we query the user that needs to be added so that we can get their public encrypt key
    let query = crate::query_command(Arc::clone(&registry), username, auth).await?;
    
    // Make the create request and fire it over to the authentication server
    let create = GroupAddRequest {
        group,
        referrer: read_key.clone(),
        who: query.advert.encrypt,
        purpose,
    };

    let response: Result<GroupAddResponse, InvokeError<GroupAddFailed>> = chain.invoke(create).await;
    match response {
        Err(InvokeError::Reply(GroupAddFailed::NoMasterKey)) => Err(GroupAddError::NoMasterKey),
        Err(InvokeError::Reply(GroupAddFailed::NoAccess)) => Err(GroupAddError::NoAccess),
        result => {
            let result = result?;
            debug!("key: {}", result.key);
            Ok(result)
        }
    }
}

pub async fn main_group_add(
    name: Option<String>,
    username: Option<String>,
    purpose: Option<AteRolePurpose>,
    auth: Url,
    session: &AteSession
) -> Result<(), GroupAddError>
{
    let name = match name {
        Some(a) => a,
        None => {
            print!("Group: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid name");
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
                Err(err) => { return Err(GroupAddError::InvalidPurpose(err.to_string())); }
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

    // Create a user using the authentication server which will give us a session with all the tokens
    let result = group_add_command(name, purpose, username, auth, session).await?;

    println!("Group user added (id={})", result.key);

    Ok(())
}