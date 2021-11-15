#![allow(unused_imports)]
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use ::ate::error::*;
use ::ate::prelude::*;
use ::ate::time::TimeKeeper;

use crate::helper::*;
use crate::model::*;
use crate::request::*;

pub struct AuthService {
    pub auth_url: url::Url,
    pub master_session: AteSessionUser,
    pub web_key: EncryptKey,
    pub edge_key: EncryptKey,
    pub contract_key: EncryptKey,
    pub time_keeper: TimeKeeper,
    pub terms_and_conditions: Option<String>,
    pub registry: Arc<Registry>,
}

impl AuthService {
    pub async fn new(
        cfg: &ConfAte,
        auth_url: url::Url,
        auth_session: AteSessionUser,
        web_key: EncryptKey,
        edge_key: EncryptKey,
        contract_key: EncryptKey,
        terms_and_conditions: Option<String>,
    ) -> Result<Arc<AuthService>, TimeError> {
        let service = Arc::new(AuthService {
            auth_url,
            master_session: auth_session,
            web_key,
            edge_key,
            contract_key,
            time_keeper: TimeKeeper::new(cfg, 30000).await?,
            registry: Registry::new(cfg)
                .await
                .temporal(true)
                .keep_alive(Duration::from_secs(60))
                .cement(),
            terms_and_conditions,
        });
        Ok(service)
    }
}

pub async fn service_auth_handlers(
    cfg: &ConfAte,
    cmd_session: AteSessionUser,
    auth_url: url::Url,
    auth_session: AteSessionUser,
    web_key: EncryptKey,
    edge_key: EncryptKey,
    contract_key: EncryptKey,
    terms_and_conditions: Option<String>,
    chain: &Arc<Chain>,
) -> Result<(), TimeError> {
    let service = AuthService::new(
        cfg,
        auth_url,
        auth_session,
        web_key,
        edge_key,
        contract_key,
        terms_and_conditions,
    )
    .await?;
    chain.add_service(&cmd_session, service.clone(), AuthService::process_login);
    chain.add_service(&cmd_session, service.clone(), AuthService::process_sudo);
    chain.add_service(&cmd_session, service.clone(), AuthService::process_reset);
    chain.add_service(
        &cmd_session,
        service.clone(),
        AuthService::process_create_user,
    );
    chain.add_service(
        &cmd_session,
        service.clone(),
        AuthService::process_create_group,
    );
    chain.add_service(&cmd_session, service.clone(), AuthService::process_query);
    chain.add_service(&cmd_session, service.clone(), AuthService::process_gather);
    chain.add_service(
        &cmd_session,
        service.clone(),
        AuthService::process_group_user_add,
    );
    chain.add_service(
        &cmd_session,
        service.clone(),
        AuthService::process_group_user_remove,
    );
    chain.add_service(
        &cmd_session,
        service.clone(),
        AuthService::process_group_details,
    );
    chain.add_service(
        &cmd_session,
        service.clone(),
        AuthService::process_group_remove,
    );
    Ok(())
}
