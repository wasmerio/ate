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

pub async fn create_group_command(registry: &Registry, group: String, auth: Url, username: String) -> Result<CreateGroupResponse, CreateError>
{
    // Open a command chain
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
    hint_group: &str
) -> Result<(String, String), CreateError>
{
    let group = match group {
        Some(a) => a,
        None => {
            #[cfg(not(feature = "force_tty"))]
            if !atty::is(atty::Stream::Stdin) {
                bail!(CreateErrorKind::InvalidArguments);
            }

            print!("{}: ", hint_group);
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid group");
            s.trim().to_string()
        }
    };

    let username = match username {
        Some(a) => a,
        None => {
            #[cfg(not(feature = "force_tty"))]
            if !atty::is(atty::Stream::Stdin) {
                bail!(CreateErrorKind::InvalidArguments);
            }

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
    hint_group: &str
) -> Result<AteSessionGroup, CreateError>
{
    let (group, username) = main_create_group_prelude(group, username, hint_group).await?;

    // Create a user using the authentication server which will give us a session with all the tokens
    let registry = ate::mesh::Registry::new( &conf_cmd()).await.cement();
    let result = match create_group_command(&registry, group, auth, username).await {
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

    println!("{} created (id={})", hint_group, result.key);
    Ok(result.session)
}