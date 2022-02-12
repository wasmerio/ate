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
use ate_files::repo::RepositorySessionFactory;
use atessh::NativeFiles;
use atessh::term_lib::api::System;
use atessh::term_lib::api::SystemAbiExt;
use tokera::model::InstanceHello;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;
use tokera::model::MasterAuthority;
use tokera::model::ServiceInstance;
use tokera::model::INSTANCE_ROOT_ID;
use tokera::model::MASTER_AUTHORITY_ID;
use atessh::term_lib;
use term_lib::bin_factory::CachedCompiledModules;
use term_lib::api::ConsoleRect;
use ate_auth::cmd::gather_command;
use ate_auth::helper::b64_to_session;

use crate::session::Session;

pub struct Server
{
    pub system: System,
    pub registry: Arc<Registry>,
    pub repo: Arc<Repository>,
    pub db_url: url::Url,
    pub auth_url: url::Url,
    pub native_files: NativeFiles,
    pub compiler: term_lib::eval::Compiler,
    pub compiled_modules: Arc<CachedCompiledModules>,
}

impl Server
{
    pub async fn new(
        db_url: Url,
        auth_url: Url,
        instance_authority: String,
        token_path: String,
        registry: Arc<Registry>,
        native_files: NativeFiles,
        compiler: term_lib::eval::Compiler,
        compiled_modules: Arc<CachedCompiledModules>
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ttl = Duration::from_secs(60);

        // Build a session factory that will load the session for this instance using the broker key
        let session_factory = SessionFactory {
            db_url: db_url.clone(),
            auth_url: auth_url.clone(),
            registry: registry.clone(),
            token_path: token_path.clone(),
            instance_authority: instance_authority.clone(),
            edge_session_cache: Arc::new(tokio::sync::Mutex::new(None)),
        };

        let repo = Repository::new(
            &registry,
            db_url.clone(),
            auth_url.clone(),
            Box::new(session_factory),
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
        let hello_buf = rx.read_buf_with_header(&wire_encryption, &mut _total_read).await?;
        let hello_instance: InstanceHello = serde_json::from_slice(&hello_buf[..])?;
        debug!("accept-web-socket: {}", hello_instance);

        // Open the instance chain that backs this particular instance
        let accessor = self.repo.get_accessor(&hello_instance.chain, hello_instance.owner_identity.as_str()).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;
        trace!("loaded file accessor for {}", hello_instance.chain);

        // Load the service instance object
        let _chain = accessor.chain.clone();
        let chain_dio = accessor.dio.clone().as_mut().await;
        trace!("loading service instance with key {}", PrimaryKey::from(INSTANCE_ROOT_ID));
        let service_instance = chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;

        // Get the native files
        trace!("loading and attaching native files");
        let native_files = self.native_files
            .get()
            .await
            .map_err(|err| {
                CommsErrorKind::InternalError(err.to_string())
            })?;

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
            native_files,
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

struct SessionFactory
{
    db_url: url::Url,
    auth_url: url::Url,
    token_path: String,
    registry: Arc<Registry>,
    instance_authority: String,
    edge_session_cache: Arc<tokio::sync::Mutex<Option<AteSessionGroup>>>,
}

#[async_trait]
impl RepositorySessionFactory
for SessionFactory
{
    async fn create(&self, sni: String, key: ChainKey) -> Result<AteSessionType, AteError>
    {
        let edge_session = {
            let mut edge_session_cache = self.edge_session_cache.lock().await;
            if edge_session_cache.is_none() {
                // First we need to get the edge session that has the rights to
                // access this domain
                let path = shellexpand::tilde(self.token_path.as_str()).to_string();
                let session = if let Ok(token) = std::fs::read_to_string(path) {
                    b64_to_session(token)
                } else {
                    let err: ate_auth::error::GatherError = ate_auth::error::GatherErrorKind::NoMasterKey.into();
                    return Err(err.into());
                };

                // Now we gather the rights to the instance domain that is capable of running these instances
                let edge_session = gather_command(
                    &self.registry,
                    self.instance_authority.clone(),
                    session.clone_inner(),
                    self.auth_url.clone(),
                ).await?;
                edge_session_cache.replace(edge_session);
            }
            edge_session_cache.clone().unwrap()
        };

        // Now we read the chain of trust and attempt to get the master authority object
        let chain = self.registry.open(&self.db_url, &key).await?;
        let dio = chain.dio(&edge_session).await;
        let master_authority = dio.load::<MasterAuthority>(&PrimaryKey::from(MASTER_AUTHORITY_ID)).await?;

        error!("{}", edge_session);

        use std::ops::Deref;
        error!("{}", serde_json::to_string_pretty(master_authority.deref()));

        // Get the private key and use it to access the authority for this chain
        let access_key =  if let Some(key) = edge_session
            .private_read_keys(AteSessionKeyCategory::AllKeys)
            .filter(|k| k.hash() == master_authority.inner_broker.ek_hash())
            .next() {
            key.clone()
        } else {
            error!("failed to get the broker key from the master edge session");
            let err: ate_auth::error::GatherError = ate_auth::error::GatherErrorKind::NoMasterKey.into();
            return Err(err.into());
        };
        let master_authority = master_authority.inner_broker.unwrap(&access_key)?;

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