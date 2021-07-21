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
    pub async fn process_group_details<'a>(&self, request: GroupDetailsRequest, context: InvocationContext<'a>) -> Result<GroupDetailsResponse, ServiceError<GroupDetailsFailed>>
    {
        debug!("group ({}) details", request.group);
        
        // Compute which chain the group should exist within
        let group_chain_key = auth_chain_key("auth".to_string(), &request.group);
        let chain = context.repository.open_by_key(&group_chain_key).await?;

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let mut dio = chain.dio(&self.master_session).await;
        let group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError::NotFound(_)) => {
                return Err(ServiceError::Reply(GroupDetailsFailed::GroupNotFound));
            },
            Err(LoadError::TransformationError(TransformError::MissingReadKey(_))) => {
                return Err(ServiceError::Reply(GroupDetailsFailed::NoMasterKey));
            },
            Err(err) => {
                return Err(ServiceError::LoadError(err));
            }
        };       

        // Check that we actually have the rights to view the details of this group
        let has_access = match &request.session {
            Some(session) => {
                let hashes = session.private_read_keys().map(|k| k.hash()).collect::<Vec<_>>();
                group.roles.iter().filter(|r| r.purpose == AteRolePurpose::Owner || r.purpose == AteRolePurpose::Delegate)
                    .any(|r| {
                        for hash in hashes.iter() {
                            if r.access.exists(hash) {
                                return true;
                            }
                        }
                        return false;
                    })
            },
            None => false
        };

        // Build the list of roles in this group
        let mut roles = Vec::new();
        for role in group.roles.iter() {
            roles.push(GroupDetailsRoleResponse {
                purpose: role.purpose.clone(),
                name: role.purpose.to_string(),
                read: role.read.clone(),
                private_read: role.private_read.clone(),
                write: role.write.clone(),
                hidden: has_access == false,
                members: match has_access {
                    true => role.access.meta_list().map(|m| m.clone()).collect::<Vec<_>>(),
                    false => Vec::new(),
                }
            });
        }

        // Return success to the caller
        Ok(GroupDetailsResponse {
            key: group.key().clone(),
            name: group.name.clone(),
            gid: group.gid,
            roles,
        })
    }
}

pub async fn group_details_command(group: String, auth: Url, session: Option<&AteSession>) -> Result<GroupDetailsResponse, GroupDetailsError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_cmd(), true).await;
    let chain = Arc::clone(&registry).open_by_url(&chain_url).await?;
    
    // Make the create request and fire it over to the authentication server
    let create = GroupDetailsRequest {
        group,
        session: session.map(|s| s.clone()),
    };

    let response: Result<GroupDetailsResponse, InvokeError<GroupDetailsFailed>> = chain.invoke(create).await;
    match response {
        Err(InvokeError::Reply(GroupDetailsFailed::NoAccess)) => Err(GroupDetailsError::NoAccess),
        Err(InvokeError::Reply(GroupDetailsFailed::GroupNotFound)) => Err(GroupDetailsError::GroupNotFound),
        result => {
            let result = result?;
            debug!("key: {}", result.key);
            Ok(result)
        }
    }
}

pub async fn main_group_details(
    group: Option<String>,
    auth: Url,
    session: Option<&AteSession>
) -> Result<(), GroupDetailsError>
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

    // Looks up the details of a group and prints them to the console
    let result = group_details_command(group, auth, session).await?;

    println!("# Group Details");
    println!("");
    println!("Key: {}", result.key);
    println!("Name: {}", result.name);
    println!("GID: {}", result.gid);
    println!("");
    println!("# Roles");
    println!("");
    for role in result.roles {
        println!("## {}", role.name);
        println!("");
        println!("read: {}", role.read);
        println!("pread: {}", role.private_read);
        println!("write: {}", role.write);
        println!("");
        if role.hidden {
            println!("[membership hidden]")
        } else {
            println!("[membership]");
            for member in role.members {
                println!("- {}", member);
            }
        }
        println!("");
    }
    Ok(())
}