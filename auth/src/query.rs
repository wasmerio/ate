#![allow(unused_imports)]
use log::{info, error, debug};
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
    pub async fn process_query<'a>(&self, request: QueryRequest, context: InvocationContext<'a>) -> Result<QueryResponse, ServiceError<QueryFailed>>
    {
        info!("query user: {}", request.email);

        // Compute which chain the user should exist within
        let user_chain_key = auth_chain_key("auth".to_string(), &request.email);
        let chain = context.repository.open_by_key(&user_chain_key).await?;
        let mut dio = chain.dio(&self.master_session).await;

        // If it does not exist then fail
        let user_key_entropy = format!("advert@{}", request.email).to_string();
        let user_key = PrimaryKey::from(user_key_entropy);
        if dio.exists(&user_key).await == false {
            return Err(ServiceError::Reply(QueryFailed::NotFound));
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
    let chain_url = crate::helper::command_url(auth.clone());
    let chain = registry.open_by_url(&chain_url).await?;
    
    // Create the query command
    let query = QueryRequest {
        email: username.clone(),
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<QueryResponse, InvokeError<QueryFailed>> = chain.invoke(query).await;
    match response {
        Err(InvokeError::Reply(QueryFailed::Banned)) => Err(QueryError::Banned),
        Err(InvokeError::Reply(QueryFailed::Suspended)) => Err(QueryError::Suspended),
        Err(InvokeError::Reply(QueryFailed::NotFound)) => Err(QueryError::NotFound),
        result => {
            let result = result?;
            //debug!("advert: {:?}", result.advert);
            Ok(result)
        }
    }
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


    let registry = ate::mesh::Registry::new(&conf_auth(), true).await;
    let result = query_command(registry, username, auth).await?;
    Ok(result.advert)
}