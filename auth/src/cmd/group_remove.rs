#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use ate::prelude::*;
use std::sync::Arc;
use url::Url;
use std::io::stdout;
use std::io::Write;

use crate::prelude::*;
use crate::helper::*;
use crate::error::*;
use crate::request::*;
use crate::opt::*;
use crate::cmd::*;

pub async fn group_remove_command(registry: &Registry, session: &AteSessionGroup, auth: Url) -> Result<GroupRemoveResponse, GroupRemoveError>
{
    // Open a command chain
    let group = session.identity().to_string();
    let chain = registry.open_cmd(&auth).await?;

    // Make the remove request and fire it over to the authentication server
    let create = GroupRemoveRequest {
        group,
        session: session.clone(),
    };

    let response: Result<GroupRemoveResponse, GroupRemoveFailed> = chain.invoke(create).await?;
    let result = response?;
    debug!("key: {}", result.key);
    Ok(result)
}

pub async fn main_group_remove(
    auth: Url,
    session: &AteSessionGroup,
    hint_group: &str
) -> Result<(), GroupRemoveError>
{
    // Remove a user from a group using the authentication server
    let registry = ate::mesh::Registry::new( &conf_cmd()).await.cement();
    let result = group_remove_command(&registry, &session, auth).await?;

    println!("{} removed (id={})", hint_group, result.key);

    Ok(())
}