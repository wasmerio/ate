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
use ate::utils::chain_key_4hex;

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

        // First we query the user that needs to be added so that we can get their public encrypt key
        let advert = match Arc::clone(&self).process_query(QueryRequest {
            identity: request.identity.clone(),
        }).await {
            Ok(a) => Ok(a),
            Err(QueryFailed::Banned) => Err(CreateGroupFailed::OperatorBanned),
            Err(QueryFailed::NotFound) => Err(CreateGroupFailed::OperatorNotFound),
            Err(QueryFailed::Suspended) => Err(CreateGroupFailed::AccountSuspended),
            Err(QueryFailed::InternalError(code)) => Err(CreateGroupFailed::InternalError(code)),
        }?.advert;

        // Extract the read key(s) from the query
        let request_nominal_read_key = advert.nominal_encrypt;
        let request_sudo_read_key = advert.sudo_encrypt;

        // Make sure the group matches the regex and is valid
        let regex = Regex::new("^/{0,1}([a-zA-Z0-9_\\.\\-]{1,})$").unwrap();
        if let Some(_captures) = regex.captures(request.group.as_str()) {
            if request.group.len() <= 0 {
                return Err(CreateGroupFailed::InvalidGroupName("the group name you specified is not long enough".to_string()));
            }
        } else {
            return Err(CreateGroupFailed::InvalidGroupName("the group name you specified is invalid".to_string()));
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
        let key_size = request_nominal_read_key.size();
        let master_key = match self.master_key() {
            Some(a) => a,
            None => { return Err(CreateGroupFailed::NoMasterKey); }
        };
        
        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key).await?;
        let dio = chain.dio_mut(&self.master_session).await;

        // Try and find a free GID
        let gid_offset = u32::MAX as u64;
        let mut gid = None;
        for n in 0u32..50u32 {
            let mut gid_test = estimate_group_name_as_gid(request.group.clone());
            if gid_test < 1000 { gid_test = gid_test + 1000; }
            gid_test = gid_test + n;
            if dio.exists(&PrimaryKey::from(gid_test as u64)).await {
                continue;
            }
            if dio.exists(&PrimaryKey::from(gid_test as u64 + gid_offset)).await {
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
            return Err(CreateGroupFailed::AlreadyExists("the group with this name already exists".to_string()));
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

        // Generate the broker encryption keys used to extend trust without composing
        // the confidentiality of the chain through wide blast radius
        let broker_read = PrivateEncryptKey::generate(key_size);
        let broker_write = PrivateSignKey::generate(key_size);

        // We generate a derived contract encryption key which we will give back to the caller
        let contract_read_key_entropy = AteHash::from_bytes(request.identity.as_bytes());
        let contract_read_key = match self.compute_super_key_from_hash(contract_read_key_entropy) {
            Some(a) => a,
            None => {
                warn!("no master key - failed to create composite key");
                return Err(CreateGroupFailed::NoMasterKey);
            }
        };
        let finance_read = contract_read_key;
        let finance_private_read = PrivateEncryptKey::generate(key_size);
        let finance_write = PrivateSignKey::generate(key_size);

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
            broker_read: broker_read.clone(),
            broker_write: broker_write.clone(),
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
                AteRolePurpose::Finance,
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
                    AteRolePurpose::Finance => {
                        role_read = finance_read.clone();
                        role_private_read = finance_private_read.clone();
                        role_write = finance_write.clone();
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
                    access.add(&request_sudo_read_key, request.identity.clone(), &owner_private_read)?;
                } else if let AteRolePurpose::Finance = purpose {
                    access.add(&request_sudo_read_key, "finance".to_string(), &owner_private_read)?;
                } else if let AteRolePurpose::Delegate = purpose {
                    access.add(&request_nominal_read_key, request.identity.clone(), &owner_private_read)?;
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
                    private_read: role_private_read.as_public_key().clone(),
                    write: role_write.as_public_key().clone(),
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

        // Create the advert object and save it using public read
        let advert_key_entropy = format!("advert@{}", request.group.clone()).to_string();
        let advert_key = PrimaryKey::from(advert_key_entropy);
        let advert = Advert {
            identity: request.group.clone(),
            id: AdvertId::GID(gid),
            nominal_encrypt: observer_private_read.as_public_key().clone(),
            nominal_auth: contributor_write.as_public_key().clone(),
            sudo_encrypt: owner_private_read.as_public_key().clone(),
            sudo_auth: owner_write.as_public_key().clone(),
            broker_auth: broker_write.as_public_key().clone(),
            broker_encrypt: broker_read.as_public_key().clone(),
        };
        let mut advert = dio.store_with_key(advert, advert_key.clone())?;
        advert.auth_mut().read = ReadOption::Everyone(None);
        advert.auth_mut().write = WriteOption::Inherit;

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
    let chain = registry.open_cmd(&auth).await?;

    // Make the create request and fire it over to the authentication server
    let create = CreateGroupRequest {
        group,
        identity: username.clone(),
    };

    let response: Result<CreateGroupResponse, CreateGroupFailed> = chain.invoke(create).await?;
    let result = response?;
    debug!("key: {}", result.key);
    Ok(result)
}

pub async fn main_create_group_prelude(
    group: Option<String>,
    username: Option<String>,
    group_hint: &str,
) -> Result<(String, String), CreateError>
{
    let group = match group {
        Some(a) => a,
        None => {
            print!("{}: ", group_hint);
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

    Ok((group, username))
}

pub async fn main_create_group(
    group: Option<String>,
    auth: Url,
    username: Option<String>,
    group_hint: &str,
) -> Result<AteSession, CreateError>
{
    let (group, username) = main_create_group_prelude(group, username, group_hint).await?;

    // Create a user using the authentication server which will give us a session with all the tokens
    let result = match create_group_command(group, auth, username).await {
        Ok(a) => a,
        Err(CreateError(CreateErrorKind::OperatorBanned, _)) => {
            eprintln!("Failed as the callers account is currently banned");
            std::process::exit(1);
        },
        Err(CreateError(CreateErrorKind::OperatorNotFound, _)) => {
            eprintln!("Failed as the callers account could not be found");
            std::process::exit(1);
        },
        Err(CreateError(CreateErrorKind::AccountSuspended, _)) => {
            eprintln!("Failed as the callers account is currently suspended");
            std::process::exit(1);
        },
        Err(CreateError(CreateErrorKind::ValidationError(reason), _)) => {
            eprintln!("{}", reason);
            std::process::exit(1);
        },
        Err(CreateError(CreateErrorKind::AlreadyExists(msg), _)) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        },
        Err(CreateError(CreateErrorKind::InvalidName(msg), _)) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        },
        Err(err) => {
            bail!(err);
        }
    };

    println!("Group created (id={})", result.key);
    Ok(result.session)
}