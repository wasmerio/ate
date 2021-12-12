use crate::error::*;
use crate::opt::OptsBus;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::fuse::*;
use wasm_bus::prelude::*;
use tokio::sync::mpsc;
use std::sync::Arc;
use ate::prelude::*;
use ate_files::prelude::*;
use ate_auth::prelude::*;
use std::time::Duration;

pub async fn main_opts_bus(
    opts: OptsBus,
    conf: AteConfig,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), BusError> {
    info!("wasm bus initializing");

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

    // Register all the functions
    listen(move |handle: CallHandle, mount: Mount|
    {
        // Derive the group from the mount address
        let mut group = None;
        if let Some((group_str, _)) = mount.name.split_once("/") {
            group = Some(group_str.to_string());
        }

        let remote = opts.remote.clone();
        let registry = registry.clone();
        let token_path = token_path.clone();
        let auth_url = auth_url.clone();
        async move
        {
            // Load the session
            let session_user = match main_session_user(None, Some(token_path.clone()), Some(auth_url)).await {
                Ok(a) => a,
                Err(err) => {
                    warn!("failed to acquire token - {}", err);
                    return;
                }
            };
            
            // Attempt to grab additional permissions for the group (if it has any)
            let session: AteSessionType = if group.is_some() {
                match main_gather(
                    group.clone(),
                    session_user.clone().into(),
                    auth_url,
                    "Group",
                )
                .await
                {
                    Ok(a) => a.into(),
                    Err(err) => {
                        debug!("Group authentication failed: {} - falling back to user level authorization", err);
                        session_user.into()
                    }
                }
            } else {
                session_user.into()
            };

            // Load the chain
            let key = ChainKey::from(mount.name);
            let chain = match registry.open(&remote, &key).await {
                Ok(a) => a,
                Err(err) => {
                    warn!("failed to open chain - {}", err);
                    return;
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

            // Add all the operations
            {
                let accessor = accessor.clone();
                respond_to(
                    handle,
                    move |_handle, meta: ReadSymlinkMetadata| {
                        let accessor = accessor.clone();
                        async move {
                            let context = RequestContext::default();
                            if let Ok(Some(file)) = accessor.search(&context, meta.path.as_str()).await {
                                info!("we made it! - META (path={}) - found", meta.path);
                            } else {
                                info!("we made it! - META (path={}) - missing", meta.path);
                            }
                            
                        }
                    },
                );
            }

            // We are now running
            info!("successfully mounted {}", mount.name);

            // The mount will shutdown when an Unmount command is received
            let (tx_unmount, mut rx_unmount) = mpsc::channel::<()>(1); 
            respond_to(
                handle,
                move |_handle, _meta: Unmount| {
                    let tx = tx_unmount.clone();
                    async move {
                        let _ = tx.send(()).await;
                    }
                },
            );
            let _ = rx_unmount.recv().await;
        }
    });

    // Enter a polling loop
    serve();
    Ok(())
}
