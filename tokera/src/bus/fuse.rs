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
use wasm_bus_fuse::api;
use wasm_bus_fuse::prelude::*;
use wasm_bus_tok::prelude::*;

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
    pub async fn listen(
        opts: Arc<OptsBus>,
        registry: Arc<Registry>,
        session_user: AteSessionUser,
        conf: AteConfig,
        auth_url: url::Url,
    ) -> Result<(), crate::error::BusError> {       

        // Register so we can respond to calls
        let server = Arc::new(FuseServer {
            registry,
            opts,
            conf,
            session_user,
            auth_url,
        });
        api::FuseService::listen(server);
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

        // Create a progress bar loader
        let progress_local = DummyLoader::default();
        let progress_remote = LoadProgress::new(std::io::stdout());
        println!("Loading the chain-of-trust");

        // Load the chain
        let remote = crate::prelude::origin_url(&remote, "db");
        let key = ChainKey::from(name.clone());
        let chain = match self
            .registry
            .open_ext(&remote, &key, progress_local, progress_remote)
            .await
        {
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
        let _ = std::io::stdout().flush();

        // Create the file system
        Ok(Arc::new(FileSystem::new(accessor, context)))
    }
}
