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
use crate::model::Advert;
use crate::opt::*;
use crate::prelude::*;
use crate::request::*;

pub async fn query_command(
    registry: &Registry,
    username: String,
    auth: Url,
) -> Result<QueryResponse, QueryError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Create the query command
    let query = QueryRequest {
        identity: username.clone(),
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<QueryResponse, QueryFailed> = chain.invoke(query).await?;
    let result = response?;
    //debug!("advert: {:?}", result.advert);
    Ok(result)
}

pub async fn main_query(username: Option<String>, auth: Url) -> Result<Advert, QueryError> {
    let username = match username {
        Some(a) => a,
        None => {
            print!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let result = query_command(&registry, username, auth).await?;
    Ok(result.advert)
}
