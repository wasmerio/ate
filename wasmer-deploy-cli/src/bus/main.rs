#![allow(unused_imports, dead_code)]
use crate::opt::OptsBus;
use ate::prelude::*;
use wasmer_auth::prelude::*;
use ate_files::codes::*;
use ate_files::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_bus::abi::CallHandle;
use wasmer_bus::abi::SerializationFormat;
use wasmer_bus::task::listen;
use wasmer_bus::task::respond_to;
use wasmer_bus::task::serve;
use wasmer_bus_fuse::api;
use wasmer_bus_fuse::api::FuseService;
use wasmer_bus_fuse::prelude::*;

use super::fuse::FuseServer;
use super::server::DeployServer;

pub async fn main_opts_bus(
    opts: OptsBus,
    conf: AteConfig,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), crate::error::BusError> {
    info!("wasmer-bus initializing");

    // Freeze the opts
    let opts = Arc::new(opts);

    // Load the session
    info!("loading the user session");
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

    // Start the fuse and tok implementations
    debug!("listing for wasmer commands");
    DeployServer::listen(opts.clone(), registry.clone(), session_user.clone(), conf.clone(), auth_url.clone()).await?;
    debug!("listing for fuse commands");
    FuseServer::listen(opts.clone(), registry.clone(), session_user.clone(), conf.clone(), auth_url.clone()).await?;
    debug!("switching to request serving mode");
    wasmer_bus::task::serve().await;
    Ok(())
}

fn conv_file_type(kind: ate_files::api::FileKind) -> api::FileType {
    let mut ret = api::FileType::default();
    match kind {
        ate_files::api::FileKind::Directory => {
            ret.dir = true;
        }
        ate_files::api::FileKind::RegularFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::FixedFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::SymLink => {
            ret.symlink = true;
        }
    }
    ret
}

fn conv_meta(file: ate_files::attr::FileAttr) -> api::Metadata {
    api::Metadata {
        ft: conv_file_type(file.kind),
        accessed: file.accessed,
        created: file.created,
        modified: file.updated,
        len: file.size,
    }
}
