#![allow(unused_imports)]
use log::{info, error, debug};
use async_trait::async_trait;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::*;
use ate::time::NtpWorker;

use crate::commands::*;
use crate::helper::*;
use crate::model::*;

#[derive(Debug)]
pub(crate) struct AuthService
{
    pub master_session: AteSession,
    pub(crate) ntp_worker: Arc<NtpWorker>,
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
impl ServiceHandler<CreateUserRequest, CreateUserResponse, CreateUserFailed>
for AuthService
{
    async fn process<'a>(&self, request: CreateUserRequest, context: InvocationContext<'a>) -> Result<CreateUserResponse, ServiceError<CreateUserFailed>>
    {
        self.process_create_user(request, context).await
    }
}

#[async_trait]
impl ServiceHandler<CreateGroupRequest, CreateGroupResponse, CreateGroupFailed>
for AuthService
{
    async fn process<'a>(&self, request: CreateGroupRequest, context: InvocationContext<'a>) -> Result<CreateGroupResponse, ServiceError<CreateGroupFailed>>
    {
        self.process_create_group(request, context).await
    }
}

#[async_trait]
impl ServiceHandler<QueryRequest, QueryResponse, QueryFailed>
for AuthService
{
    async fn process<'a>(&self, request: QueryRequest, context: InvocationContext<'a>) -> Result<QueryResponse, ServiceError<QueryFailed>>
    {
        self.process_query(request, context).await
    }
}

#[async_trait]
impl ServiceHandler<GatherRequest, GatherResponse, GatherFailed>
for AuthService
{
    async fn process<'a>(&self, request: GatherRequest, context: InvocationContext<'a>) -> Result<GatherResponse, ServiceError<GatherFailed>>
    {
        self.process_gather(request, context).await
    }
}


pub async fn service_logins(cfg: &ConfAte, cmd_session: AteSession, auth_session: AteSession, chain: &Arc<Chain>)
-> Result<(), TimeError>
{
    let service = Arc::new(
        AuthService
        {
            master_session: auth_session,
            ntp_worker:  NtpWorker::create(cfg, 30000).await?
        }
    );

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<LoginRequest, LoginResponse, LoginFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<CreateUserRequest, CreateUserResponse, CreateUserFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<CreateGroupRequest, CreateGroupResponse, CreateGroupFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<QueryRequest, QueryResponse, QueryFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<GatherRequest, GatherResponse, GatherFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    Ok(())
}