#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::LoadError;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::helper::*;

impl AuthService
{
    pub async fn process_query(self: Arc<Self>, request: QueryRequest) -> Result<QueryResponse, QueryFailed>
    {
        info!("query user/group: {}", request.identity);

        // Compute which chain the user should exist within
        let user_chain_key = chain_key_4hex(&request.identity, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &user_chain_key).await?;
        let dio = chain.dio(&self.master_session).await;

        // If it does not exist then fail
        let user_key_entropy = format!("advert@{}", request.identity).to_string();
        let user_key = PrimaryKey::from(user_key_entropy);
        if dio.exists(&user_key).await == false {
            return Err(QueryFailed::NotFound);
        }

        // Load the advert
        let advert = dio.load::<Advert>(&user_key).await?;

        // Return success to the caller
        Ok(QueryResponse {
            advert: advert.take(),
        })
    }
}

pub async fn query_command(registry: Arc<ate::mesh::Registry>, username: String, auth: Url) -> Result<QueryResponse, QueryError>
{
    // Open a command chain
    let chain = registry.open(&auth, &chain_key_cmd()).await?;
    
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

pub async fn main_query(
    username: Option<String>,
    auth: Url
) -> Result<Advert, QueryError>
{
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


    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let result = query_command(registry, username, auth).await?;
    Ok(result.advert)
}