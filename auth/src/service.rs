#![allow(unused_imports)]
use log::{info, error, debug};
use async_trait::async_trait;
use std::sync::Arc;
use ate::prelude::*;

use ate::error::*;
use crate::commands::*;

#[derive(Debug, Default)]
struct AuthService
{
}

#[async_trait]
impl ServiceHandler<LoginRequest, LoginResponse>
for AuthService
{
    async fn process<'a>(&self, request: LoginRequest, _context: InvocationContext<'a>) -> LoginResponse
    {
        info!("login attempt: {}", request.email);
        LoginResponse::AccountLocked
    }   
}

pub async fn service_logins(session: AteSession, chain: &Arc<Chain>)
{
    chain.add_service(session, Arc::new(AuthService::default()))
}