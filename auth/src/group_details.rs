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
    pub async fn process_group_details(self: Arc<Self>, request: GroupDetailsRequest) -> Result<GroupDetailsResponse, GroupDetailsFailed>
    {
        debug!("group ({}) details", request.group);
        
        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key).await?;

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let dio = chain.dio(&self.master_session).await;
        let group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                return Err(GroupDetailsFailed::GroupNotFound);
            },
            Err(LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)), _)) => {
                return Err(GroupDetailsFailed::NoMasterKey);
            },
            Err(err) => {
                bail!(err);
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
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let chain = registry.open_cmd(&auth).await?;
    
    // Make the create request and fire it over to the authentication server
    let create = GroupDetailsRequest {
        group,
        session: session.map(|s| s.clone()),
    };

    let response: Result<GroupDetailsResponse, GroupDetailsFailed> = chain.invoke(create).await?;
    let result = response?;
    debug!("key: {}", result.key);
    Ok(result)
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