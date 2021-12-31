#![allow(unused_imports, dead_code)]
use async_trait::async_trait;
use ate::prelude::*;
use ate_auth::prelude::*;
use ate_files::codes::*;
use ate_files::prelude::*;
use derivative::*;
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::prelude::*;
use wasm_bus_fuse::api;
use wasm_bus_fuse::prelude::*;

use super::file_system::FileSystem;
use crate::opt::OptsBus;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FuseServer {
    #[derivative(Debug = "ignore")]
    registry: Arc<Registry>,
    #[derivative(Debug = "ignore")]
    opts: Arc<OptsBus>,
    conf: AteConfig,
    session_user: AteSessionUser,
    auth_url: url::Url,
}

impl FuseServer {
    pub async fn serve(
        opts: OptsBus,
        conf: AteConfig,
        token_path: String,
        auth_url: url::Url,
    ) -> Result<(), crate::error::BusError> {
        // Load the session
        let session_user = match main_session_user(None, Some(token_path.clone()), None).await {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to acquire token - {}", err);
                return Err(crate::error::BusErrorKind::LoginFailed.into());
            }
        };

        // Build the configuration used to access the chains
        let mut conf = conf.clone();
        conf.configured_for(opts.configured_for);
        conf.log_format.meta = opts.meta_format;
        conf.log_format.data = opts.data_format;
        conf.recovery_mode = opts.recovery_mode;
        conf.compact_mode = opts
            .compact_mode
            .with_growth_factor(opts.compact_threshold_factor)
            .with_growth_size(opts.compact_threshold_size)
            .with_timer_value(Duration::from_secs(opts.compact_timer));

        // Create the registry
        let registry = Arc::new(Registry::new(&conf).await);

        // Register so we can respond to calls
        let server = Arc::new(FuseServer {
            registry,
            opts: Arc::new(opts),
            conf,
            session_user: session_user,
            auth_url,
        });
        api::FuseService::listen(server);
        api::FuseService::serve();
        Ok(())
    }
}

#[async_trait]
impl api::FuseSimplified for FuseServer {
    async fn mount(
        &self,
        name: String,
    ) -> Result<Arc<dyn api::FileSystem + Send + Sync + 'static>, CallError> {
        // Derive the group from the mount address
        let mut group = None;
        if let Some((group_str, _)) = name.split_once("/") {
            group = Some(group_str.to_string());
        }

        // Attempt to grab additional permissions for the group (if it has any)
        let session: AteSessionType = if group.is_some() {
            match main_gather(
                group.clone(),
                self.session_user.clone().into(),
                self.auth_url.clone(),
                "Group",
            )
            .await
            {
                Ok(a) => a.into(),
                Err(err) => {
                    debug!("Group authentication failed: {} - falling back to user level authorization", err);
                    self.session_user.clone().into()
                }
            }
        } else {
            self.session_user.clone().into()
        };

        // Build the request context
        let mut context = RequestContext::default();
        context.uid = session.uid().unwrap_or_default();
        context.gid = session.gid().unwrap_or_default();

        let remote = self.opts.remote.clone();

        // Load the chain
        let key = ChainKey::from(name.clone());
        let chain = match self.registry.open(&remote, &key).await {
            Ok(a) => a,
            Err(err) => {
                warn!("failed to open chain - {}", err);
                return Err(CallError::BadRequest);
            }
        };
        let accessor = Arc::new(
            FileAccessor::new(
                chain.as_arc(),
                group,
                session,
                TransactionScope::Local,
                TransactionScope::Local,
                false,
                false,
            )
            .await,
        );

        // Create the file system
        Ok(Arc::new(FileSystem::new(accessor, context)))
    }
}
