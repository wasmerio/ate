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
    pub time_keeper: TimeKeeper,
    pub registry: Arc<Registry>,
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
                time_keeper:  TimeKeeper::new(cfg, 30000).await?,
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
    chain.add_service(cmd_session.clone(), service.clone(), service.process_login);
    chain.add_service(cmd_session.clone(), service.clone(), service.process_create_user);
    chain.add_service(cmd_session.clone(), service.clone(), service.process_create_group);
    chain.add_service(cmd_session.clone(), service.clone(), service.process_query);
    chain.add_service(cmd_session.clone(), service.clone(), service.process_gather);
    chain.add_service(cmd_session.clone(), service.clone(), service.process_group_user_add);
    chain.add_service(cmd_session.clone(), service.clone(), service.process_group_user_remove);
    chain.add_service(cmd_session.clone(), service.clone(), service.process_group_details);
    Ok(())
}