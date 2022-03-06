#![allow(unused_imports, dead_code)]
use async_trait::async_trait;
use ate::loader::DummyLoader;
use ate::prelude::*;
use ate::utils::LoadProgress;
use ate_auth::prelude::*;
use ate_files::codes::*;
use ate_files::prelude::*;
use derivative::*;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::prelude::*;
use wasm_bus_tok::api;
use wasm_bus_tok::prelude::*;
use ate_auth::cmd::query_command;
use ate_auth::request::QueryResponse;
use ate_auth::error::QueryError;
use ate_auth::error::QueryErrorKind;

use super::file_system::FileSystem;
use crate::opt::OptsBus;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TokServer {
    #[derivative(Debug = "ignore")]
    registry: Arc<Registry>,
    #[derivative(Debug = "ignore")]
    opts: Arc<OptsBus>,
    conf: AteConfig,
    session_user: AteSessionUser,
    auth_url: url::Url,
}

impl TokServer {
    pub async fn listen(
        opts: Arc<OptsBus>,
        registry: Arc<Registry>,
        session_user: AteSessionUser,
        conf: AteConfig,
        auth_url: url::Url,
    ) -> Result<(), crate::error::BusError> {
        // Register so we can respond to calls
        let server = Arc::new(TokServer {
            registry,
            opts,
            conf,
            session_user,
            auth_url,
        });
        api::TokService::listen(server);
        Ok(())
    }
}

#[async_trait]
impl api::TokSimplified for TokServer {
    async fn user_exists(
        &self,
        email: String,
    ) -> api::TokResult<bool> {
        let query = query_command(&self.registry, email, self.auth_url.clone()).await;
        match query {
            Ok(_) => Ok(true),
            Err(QueryError(QueryErrorKind::Banned, _)) => Ok(true),
            Err(QueryError(QueryErrorKind::Suspended, _)) => Ok(true),
            Err(QueryError(QueryErrorKind::NotFound, _)) => Ok(false),
            Err(QueryError(QueryErrorKind::InternalError(code), _)) => Err(api::TokError::InternalError(code)),
            Err(err) => {
                let code = ate::utils::obscure_error(err);
                Err(api::TokError::InternalError(code))
            }
        }
    }

    async fn user_create(
        &self,
        _email: String,
        _password: String
    ) -> api::TokResult<()> {
        return Err(api::TokError::NotImplemented);
    }

    async fn login(
        &self,
        _email: String,
        _password: String,
        _code: Option<String>
    ) -> Result<Arc<dyn api::Session + Send + Sync + 'static>, CallError> {
        return Err(CallError::Unsupported);
    }
}
