use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
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
use ate_files::prelude::FileAccessor;
use tokera::model::MasterAuthority;
use tokera::model::ServiceInstance;
use tokera::model::INSTANCE_ROOT_ID;
use tokera::model::MASTER_AUTHORITY_ID;
use atessh::term_lib;
use term_lib::bin_factory::CachedCompiledModules;
use term_lib::api::ConsoleRect;

use crate::session::Session;

pub struct Server
{
    pub system: System,
    pub registry: Arc<Registry>,
    pub repo: Arc<Repository>,
    pub db_url: url::Url,
    pub auth_url: url::Url,
    pub native_files: Arc<FileAccessor>,
    pub compiler: term_lib::eval::Compiler,
    pub compiled_modules: Arc<CachedCompiledModules>,
}

impl Server
{
    pub async fn new(
        db_url: Url,
        auth_url: Url,
        edge_session: AteSessionGroup,
        registry: Arc<Registry>,
        native_files: Arc<FileAccessor>,
        compiler: term_lib::eval::Compiler,
        compiled_modules: Arc<CachedCompiledModules>
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ttl = Duration::from_secs(60);

        // Get the broker key from the edge session (if we have none then fail)
        let broker_key = if let Some(broker_key) = edge_session.broker_read() {
            broker_key.clone()
        } else {
            panic!("The edge session does not contain a broker read key and hence the server can not read session instance chains.");
        };

        // Build a session factory that will load the session for this instance using the broker key
        let session_factory = {
            let db_url = db_url.clone();
            let registry = registry.clone();
            move |sni: &str, key: ChainKey| {
                let sni = sni.to_string();
                let db_url = db_url.clone();
                let edge_session = edge_session.clone();
                let broker_key = broker_key.clone();
                let registry = registry.clone();
                async move {
                    // Now we read the chain of trust and attempt to get the master authority object
                    let chain = registry.open(&db_url, &key).await?;
                    let dio = chain.dio(&edge_session).await;
                    let master_authority = dio.load::<MasterAuthority>(&PrimaryKey::from(MASTER_AUTHORITY_ID)).await?;
                    let master_authority = master_authority.inner_broker.unwrap(&broker_key)?;

                    // Build the session using the master authority
                    let mut chain_session = AteSessionUser::default();
                    chain_session.add_user_read_key(&master_authority.read);
                    chain_session.add_user_write_key(&master_authority.write);
                    chain_session.add_user_uid(0);
                    let mut chain_session = AteSessionGroup::new(AteSessionInner::User(chain_session), sni);
                    chain_session.add_group_gid(&AteRolePurpose::Observer, 0);
                    chain_session.add_group_gid(&AteRolePurpose::Contributor, 0);
                    chain_session.add_group_read_key(&AteRolePurpose::Observer, &master_authority.read);
                    chain_session.add_group_write_key(&AteRolePurpose::Contributor, &master_authority.write);
                    Ok(AteSessionType::Group(chain_session))
                }
            }
        };

        let repo = Repository::new(
            &registry,
            db_url.clone(),
            auth_url.clone(),
            session_factory,
            ttl,
        )
        .await?;

        Ok(Self {
            system: System::default(),
            db_url,
            auth_url,
            registry,
            repo,
            native_files,
            compiler,
            compiled_modules,
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
        let hello_instance: InstanceHello = serde_json::from_slice(&hello_buf[..])?;
        debug!("accept-web-socket: {}", hello_instance);

        // Open the instance chain that backs this particular instance
        let accessor = self.repo.get_accessor(&hello_instance.chain, hello_instance.owner_identity.as_str()).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;

        // Load the service instance object
        let _chain = accessor.chain.clone();
        let chain_dio = accessor.dio.clone().as_mut().await;
        let service_instance = chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;

        // Build the session
        let session = Session {
            rx,
            tx,
            hello,
            sock_addr,
            wire_encryption,
            hello_instance,
            accessor,
            service_instance,
            native_files: self.native_files.clone(),
            rect: Arc::new(Mutex::new(ConsoleRect { cols: 80, rows: 25 })),
            compiler: self.compiler.clone(),
            compiled_modules: self.compiled_modules.clone()
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