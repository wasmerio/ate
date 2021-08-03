#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use async_trait::async_trait;
use std::sync::Arc;

use ::ate::prelude::*;
use ::ate::error::*;
use ::ate::time::TimeKeeper;

use crate::commands::*;
use crate::helper::*;
use crate::model::*;

pub struct AuthService
{
    pub auth_url: url::Url,
    pub master_session: AteSession,
    pub time_keeper: Arc<TimeKeeper>,
    pub registry: Arc<Registry>,
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

#[async_trait]
impl ServiceHandler<GroupUserAddRequest, GroupUserAddResponse, GroupUserAddFailed>
for AuthService
{
    async fn process<'a>(&self, request: GroupUserAddRequest, context: InvocationContext<'a>) -> Result<GroupUserAddResponse, ServiceError<GroupUserAddFailed>>
    {
        self.process_group_user_add(request, context).await
    }
}

#[async_trait]
impl ServiceHandler<GroupUserRemoveRequest, GroupUserRemoveResponse, GroupUserRemoveFailed>
for AuthService
{
    async fn process<'a>(&self, request: GroupUserRemoveRequest, context: InvocationContext<'a>) -> Result<GroupUserRemoveResponse, ServiceError<GroupUserRemoveFailed>>
    {
        self.process_group_user_remove(request, context).await
    }
}

#[async_trait]
impl ServiceHandler<GroupDetailsRequest, GroupDetailsResponse, GroupDetailsFailed>
for AuthService
{
    async fn process<'a>(&self, request: GroupDetailsRequest, context: InvocationContext<'a>) -> Result<GroupDetailsResponse, ServiceError<GroupDetailsFailed>>
    {
        self.process_group_details(request, context).await
    }
}

impl AuthService
{
    pub async fn new(cfg: &ConfAte, auth_url: url::Url, auth_session: AteSession) -> Result<Arc<AuthService>, TimeError>
    {
        let service = Arc::new(
            AuthService
            {
                auth_url,
                master_session: auth_session,
                time_keeper:  Arc::new(TimeKeeper::new(cfg, 30000).await?),
                registry: Registry::new(cfg).await.cement(),
            }
        );
        Ok(service)
    }
}

pub async fn service_auth_handlers(cfg: &ConfAte, cmd_session: AteSession, auth_url: url::Url, auth_session: AteSession, chain: &Arc<Chain>)
-> Result<(), TimeError>
{
    let service = AuthService::new(cfg, auth_url, auth_session).await?;

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

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<GroupUserAddRequest, GroupUserAddResponse, GroupUserAddFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<GroupUserRemoveRequest, GroupUserRemoveResponse, GroupUserRemoveFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    {
        let service = Arc::clone(&service);
        let service: ServiceInstance<GroupDetailsRequest, GroupDetailsResponse, GroupDetailsFailed> = service;
        chain.add_service(cmd_session.clone(), service);
    }

    Ok(())
}