#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::{io::stdout, path::Path};
use std::io::Write;
use url::Url;
use std::ops::Deref;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::LoadError;
use ate::error::TransformError;
use ate::utils::chain_key_4hex;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::helper::*;
use super::login::main_session_start;
use super::main_session_user;
use super::main_session_sudo;
use super::main_sudo;

impl AuthService
{
    pub async fn process_gather(self: Arc<Self>, request: GatherRequest) -> Result<GatherResponse, GatherFailed>
    {
        info!("gather attempt: {}", request.group);

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let master_key = match self.master_key() {
            Some(a) => a,
            None => { return Err(GatherFailed::NoMasterKey); }
        };

        let mut super_session = AteSessionUser::default();
        super_session.user.add_read_key(&master_key);

        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key).await?;
        
        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let dio = chain.dio(&self.master_session).await;
        let group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                return Err(GatherFailed::GroupNotFound(request.group));
            },
            Err(LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)), _)) => {
                return Err(GatherFailed::NoMasterKey);
            },
            Err(err) => {
                bail!(err);
            }
        };

        // Now go into a loading loop on the session
        let session = complete_group_auth(group.deref(), request.session)?;
        
        // Return the session that can be used to access this user
        Ok(GatherResponse {
            group_name: request.group.clone(),
            gid: group.gid,
            group_key: group.key().clone(),
            authority: session
        })
    }
}

pub async fn gather_command(registry: &Arc<Registry>, group: String, session: AteSessionInner, auth: Url) -> Result<AteSessionGroup, GatherError>
{
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;
    
    // Create the gather command
    let gather = GatherRequest {
        group: group.clone(),
        session,
    };

    // Attempt the gather request with a 10 second timeout
    let response: Result<GatherResponse, GatherFailed> = chain.invoke(gather).await?;
    let result = response?;
    Ok(result.authority)
}

pub async fn main_session_group(token_string: Option<String>, token_file_path: Option<String>, group: String, sudo: bool, code: Option<String>, auth_url: Option<url::Url>) -> Result<AteSessionGroup, GatherError>
{
    let session = main_session_start(token_string, token_file_path, auth_url.clone()).await?;

    let mut session = match session {
        AteSessionType::Group(a) => {
            if a.group.name == group {
                return Ok(a);
            }
            a.inner
        },
        AteSessionType::User(a) => AteSessionInner::User(a),
        AteSessionType::Sudo(a) => AteSessionInner::Sudo(a),
    };

    if sudo {
        session = match session {
            AteSessionInner::User(a) => {
                if let Some(auth) = auth_url.clone() {
                    AteSessionInner::Sudo(main_sudo(a, code, auth).await?)
                } else {
                    AteSessionInner::User(a)
                }
            },
            a => a
        };
    }

    if let Some(auth) = auth_url {
        Ok(main_gather(Some(group), session, auth).await?)
    } else {
        Ok(AteSessionGroup::new(session, group))
    }
}

pub async fn main_gather(
    group: Option<String>,
    session: AteSessionInner,
    auth: Url
) -> Result<AteSessionGroup, GatherError>
{
    let group = match group {
        Some(a) => a,
        None => {
            eprint!("Group: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid group");
            s.trim().to_string()
        }
    };

    // Gather using the authentication server which will give us a new session with the extra tokens
    let registry = ate::mesh::Registry::new( &conf_cmd()).await.cement();
    let session = gather_command(&registry, group, session, auth).await?;
    Ok(session)
}