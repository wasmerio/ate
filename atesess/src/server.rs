use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ate::comms::HelloMetadata;
use ate::comms::StreamRoute;
use ate::comms::StreamRx;
use ate::comms::Upstream;
use ate::prelude::*;
use ate_files::repo::Repository;
use atessh::term_lib::api::System;
use atessh::term_lib::api::SystemAbiExt;
use tokera::model::InstanceHello;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use crate::session::Session;

pub struct Server
{
    pub system: System,
    pub registry: Arc<Registry>,
    pub repo: Arc<Repository>,
    pub db_url: url::Url,
    pub auth_url: url::Url,
}

impl Server
{
    pub async fn new(db_url: Url, auth_url: Url, edge_key: EncryptKey, registry: Arc<Registry>) -> Result<Self, Box<dyn std::error::Error>> {
        let ttl = Duration::from_secs(60);

        let repo = Repository::new(
            &registry,
            db_url.clone(),
            auth_url.clone(),
            "edge-read".to_string(),
            edge_key,
            ttl,
        )
        .await?;

        Ok(Self {
            system: System::default(),
            db_url,
            auth_url,
            registry,
            repo,
        })
    }
}
#[async_trait]
impl StreamRoute
for Server
{
    async fn accepted_web_socket(
        &self,
        mut rx: StreamRx,
        tx: Upstream,
        hello: HelloMetadata,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>
    {
        // Read the instance hello message
        let mut _total_read = 0u64;
        let hello_buf = rx.read_buf(&wire_encryption, &mut _total_read).await?;
        let hello_instance: InstanceHello = bincode::deserialize(&hello_buf[..])?;
        debug!("accept-web-socket: {}", hello_instance);

        // Open the instance chain that backs this particular instance
        let accessor = self.repo.get_accessor(&hello_instance.chain, hello_instance.owner_identity.as_str()).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;
        
        // Build the session
        let session = Session {
            rx,
            tx,
            hello,
            sock_addr,
            wire_encryption,
            hello_instance,
            accessor,
        };

        // Start the background thread that will process events on the session
        self.system.fork_shared(|| async move {
            if let Err(err) = session.run().await {
                debug!("instance session failed: {}", err);
            }
        });
        Ok(())
    }
}