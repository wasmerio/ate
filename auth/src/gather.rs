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

        let mut super_session = request.session.clone();
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
        let session = complete_group_auth(group.deref(), request.session.clone())?;
        
        // Return the session that can be used to access this user
        Ok(GatherResponse {
            group_name: request.group.clone(),
            gid: group.gid,
            group_key: group.key().clone(),
            authority: session
        })
    }
}

pub async fn gather_command(group: String, session: AteSession, auth: Url) -> Result<AteSession, GatherError>
{
    // Open a command chain
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let chain = registry.open(&auth, &registry.chain_key_cmd()).await?;
    
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

pub async fn main_gather(
    group: Option<String>,
    session: AteSession,
    auth: Url
) -> Result<AteSession, GatherError>
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
    let session = gather_command(group, session, auth).await?;
    Ok(session)
}