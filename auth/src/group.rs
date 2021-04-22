#![allow(unused_imports)]
use log::{info, error, debug};
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;

use ate::prelude::*;
use ate::error::LoadError;
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
        let master_key = match self.master_key() {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateGroupFailed::NoMasterKey)); }
        };

        // Access to read the 
        let super_key = match self.compute_super_key(request.read_key) {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateGroupFailed::NoMasterKey)); }
        };
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&master_key);
        super_session.user.add_read_key(&super_key);

        // Compute which chain the group should exist within
        let group_chain_key = auth_chain_key("auth".to_string(), &request.name);
        let chain = context.repository.open_by_key(&group_chain_key).await?;
        
        // If it already exists then fail
        let user_key = PrimaryKey::from(request.name.clone());
        let mut dio = chain.dio(&super_session).await;
        if dio.exists(&user_key).await {
            return Err(ServiceError::Reply(CreateGroupFailed::AlreadyExists));
        }

        // Generate the owner encryption keys used to protect this role
        let owner_read = EncryptKey::generate(KeySize::Bit256);
        let owner_write = PrivateSignKey::generate(KeySize::Bit256);

        // The super session needs the owner keys so that it can save the records
        let mut super_session = super_session.clone();
        super_session.user.add_read_key(&owner_read);
        super_session.user.add_write_key(&owner_write);
        let mut dio = chain.dio(&super_session).await;
        
        // Create the group and save it
        let group = Group {
            name: request.name,
            roles: Vec::new(),
        };
        let mut group = Dao::make(user_key.clone(), chain.default_format(), group);

        // Add the other roles
        for purpose in vec![
            AteRolePurpose::Owner,
            AteRolePurpose::Delegate,
            AteRolePurpose::Contributor,
            AteRolePurpose::Observer
        ].iter()
        {
            // Add the owner role to the group (as its a super_key the authentication server
            // is required to read the group records and load them, while the authentication
            // server can run in a distributed mode it is a centralized authority)
            let role = Role {
                purpose: purpose.clone(),
                access: match purpose {
                    RolePurpose::Owner => {
                        MultiEncryptedSecureData::new(&super_key, Authorization {
                            read: owner_read.clone(),
                            write: owner_write.clone()
                        })?
                    },
                    _ => {
                        MultiEncryptedSecureData::new(&owner_read, Authorization {
                            read: EncryptKey::generate(KeySize::Bit256),
                            write: PrivateSignKey::generate(KeySize::Bit256)
                        })?
                    }
                }
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
        let session = complete_group_auth(group.deref(), super_session.clone())?;

        // Return success to the caller
        Ok(CreateGroupResponse {
            key: group.key().clone(),
            session,
        })
    }
}

#[allow(dead_code)]
pub async fn create_group_command(name: String, auth: Url, session: &AteSession) -> Result<CreateGroupResponse, CreateError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_auth(), true).await;
    let chain = registry.open_by_url(&chain_url).await?;

    // Extract the read key from the session that will be used for the owner
    // key on the file-system
    let read_key = match session.user.read_keys().next() {
        Some(a) => a.clone(),
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
            print!("Name: ");
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