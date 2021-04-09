#![allow(unused_imports)]
use log::{info, error, debug};
use async_trait::async_trait;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::*;

use crate::commands::*;
use crate::helper::*;
use crate::model::*;

#[derive(Debug)]
pub(crate) struct AuthService
{
    pub master_session: AteSession
}

#[async_trait]
impl ServiceHandler<LoginRequest, LoginResponse, LoginFailed>
for AuthService
{
    async fn process<'a>(&self, request: LoginRequest, context: InvocationContext<'a>) -> Result<LoginResponse, ServiceError<LoginFailed>>
    {
        self.process_login(request, context).await
    }
}

#[async_trait]
impl ServiceHandler<CreateRequest, CreateResponse, CreateFailed>
for AuthService
{
    async fn process<'a>(&self, request: CreateRequest, context: InvocationContext<'a>) -> Result<CreateResponse, ServiceError<CreateFailed>>
    {
        self.process_create(request, context).await
    }
}


pub async fn service_logins(cmd_session: AteSession, auth_session: AteSession, chain: &Arc<Chain>)
{
    let service = Arc::new(
        AuthService
        {
            master_session: auth_session,
        }
    );

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<LoginRequest, LoginResponse, LoginFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<CreateRequest, CreateResponse, CreateFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }
}