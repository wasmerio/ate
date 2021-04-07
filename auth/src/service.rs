#![allow(unused_imports)]
use log::{info, error, debug};
use async_trait::async_trait;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::*;

use crate::commands::*;
use crate::helper::*;

#[derive(Debug)]
struct AuthService
{
}

#[async_trait]
impl ServiceHandler<LoginRequest, LoginResponse, LoginFailed>
for AuthService
{
    async fn process<'a>(&self, request: LoginRequest, context: InvocationContext<'a>) -> Result<LoginResponse, ServiceError<LoginFailed>>
    {
        // Compute which chain the user should exist within
        let user_url = auth_url(url::Url::parse("tcp://localhost/auth").unwrap(), &request.email);
        let _chain = context.repository.open_by_url(&user_url).await?;

        //context.repository.open_by_key()
        info!("login attempt: {}", request.email);
        Err(ServiceError::Reply(LoginFailed::AccountLocked))
    }   
}

pub async fn service_logins(session: AteSession, chain: &Arc<Chain>)
{
    chain.add_service(session, Arc::new(AuthService { }))
}