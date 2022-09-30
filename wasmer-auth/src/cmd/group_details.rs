#![allow(unused_imports)]
use ate::prelude::*;
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use crate::error::*;
use crate::helper::*;
use crate::opt::*;
use crate::prelude::*;
use crate::request::*;

pub async fn group_details_command(
    registry: &Registry,
    group: String,
    auth: Url,
    session: Option<&AteSessionGroup>,
) -> Result<GroupDetailsResponse, GroupDetailsError> {
    // Open a command chain
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
    session: Option<&AteSessionGroup>,
    hint_group: &str,
) -> Result<(), GroupDetailsError> {
    let group = match group {
        Some(a) => a,
        None => {
            print!("{}: ", hint_group);
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect(format!("Did not enter a valid {}", hint_group.to_lowercase()).as_str());
            s.trim().to_string()
        }
    };

    // Looks up the details of a group and prints them to the console
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let result = group_details_command(&registry, group, auth, session).await?;

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
