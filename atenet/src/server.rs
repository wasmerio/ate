use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;
use error_chain::bail;
#[allow(unused_imports)]
use tokio::sync::mpsc;

use async_trait::async_trait;
use ate::comms::HelloMetadata;
use ate::comms::RawStreamRoute;
use ate::comms::StreamRoute;
use ate::comms::StreamRx;
use ate::comms::Upstream;
use ate::comms::StreamReader;
use ate::prelude::*;
use ate_files::repo::Repository;
use ate_files::repo::RepositorySessionFactory;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;
use tokera::model::MasterAuthority;
use tokera::model::MASTER_AUTHORITY_ID;
#[allow(unused_imports)]
use tokera::model::InstanceCall;
use tokera::model::SwitchHello;
use ate_auth::cmd::impersonate_command;
use ate_auth::helper::b64_to_session;
use tokio::sync::RwLock;

use super::switch::*;
use super::session::*;
use super::udp::*;

pub struct Server
{
    pub registry: Arc<Registry>,
    pub repo: Arc<Repository>,
    pub udp: UdpPeer,
    pub db_url: url::Url,
    pub auth_url: url::Url,
    pub instance_authority: String,
    pub switches: RwLock<HashMap<ChainKey, Weak<Switch>>>,
}

impl Server
{
    pub async fn new(
        db_url: Url,
        auth_url: Url,
        instance_authority: String,
        token_path: String,
        registry: Arc<Registry>,
        ttl: Duration,
        udp_port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>>
    {
        // Build a switch factory that will connect clients to the current switch
        let switch_factory = SwitchFactory {
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
            Box::new(switch_factory),
            ttl,
        )
        .await?;

        let switches = RwLock::new(HashMap::default());
        let udp = UdpPeer::new(udp_port);

        Ok(Self {
            db_url,
            auth_url,
            registry,
            repo,
            instance_authority,
            switches,
            udp,
        })
    }

    pub async fn get_or_create_switch(&self, key: ChainKey) -> Result<(Arc<Switch>, bool), CommsError> {
        // Check the cache
        {
            let guard = self.switches.read().await;
            if let Some(ret) = guard.get(&key) {
                if let Some(ret) = ret.upgrade() {
                    return Ok((ret, false));
                }
            }
        }

        // Open the instance chain that backs this particular instance
        // (this will reuse accessors across threads and calls)
        let accessor = self.repo.get_accessor(&key, self.instance_authority.as_str()).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;
        trace!("loaded file accessor for {}", key);

        // Enter a write lock and check again
        let mut guard = self.switches.write().await;
        if let Some(ret) = guard.get(&key) {
            if let Some(ret) = ret.upgrade() {
                return Ok((ret, false));
            }
        }
        
        // Build the switch
        let addr = self.udp.local_ip();
        let switch = Switch::new(accessor, addr).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;

        // Cache and and return it
        guard.insert(key.clone(), Arc::downgrade(&switch));
        Ok((switch, true))
    }

    async fn accept_internal(
        &self,
        rx: Box<dyn StreamReader + Send + Sync + 'static>,
        tx: Upstream,
        hello: HelloMetadata,
        hello_switch: SwitchHello,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>
    {
        // Get or create the switch
        let key = hello_switch.chain.clone();
        let (switch, _) = self.get_or_create_switch(key).await?;
        let port = switch.new_port().await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;

        // Create the session that will process packets for this switch
        let session = Session {
            rx,
            tx,
            hello,
            hello_switch,
            sock_addr,
            wire_encryption,
            port,
        };
        
        // Start the background thread that will process events on the session
        tokio::task::spawn(async move {
            if let Err(err) = session.run().await {
                debug!("instance session failed: {}", err);
            }
        });
        Ok(())
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
        let hello_switch: SwitchHello = serde_json::from_slice(&hello_buf[..])?;
        debug!("accept-web-socket: {}", hello_switch);

        // Build the rx and tx
        let rx = Box::new(rx);

        // Accept the web connection
        self.accept_internal(
            rx,
            tx,
            hello,
            hello_switch,
            sock_addr,
            wire_encryption
        ).await
    }
}

#[async_trait]
impl RawStreamRoute
for Server
{
    async fn accepted_raw_web_socket(
        &self,
        rx: StreamRx,
        tx: Upstream,
        uri: http::Uri,
        headers: http::HeaderMap,
        sock_addr: SocketAddr,
        server_id: NodeId,
    ) -> Result<(), CommsError>
    {
        // Get the chain and the topic
        let path = std::path::PathBuf::from(uri.path().to_string());
        let chain = {
            let mut path_iter = path.iter();
            path_iter.next();
            path_iter.next()
                .map(|a| a.to_string_lossy().to_string())
                .ok_or_else(|| {
                    CommsErrorKind::InternalError(format!("instance web_socket path is invalid - {}", uri))
                })?
        };
        let chain = ChainKey::new(chain.clone());
        
        // Get the authorization
        if headers.contains_key(http::header::AUTHORIZATION) == false {
            bail!(CommsErrorKind::Refused);
        }
        let auth = headers[http::header::AUTHORIZATION].clone();

        debug!("accept-raw-web-socket: uri: {}", uri);

        // Make a fake hello from the HTTP metadata
        let hello = HelloMetadata {
            client_id: NodeId::generate_client_id(),
            server_id,
            path: path.to_string_lossy().to_string(),
            encryption: None,
            wire_format: tx.wire_format,
        };
        let hello_switch = SwitchHello {
            access_token: auth.to_str().unwrap().to_string(),
            chain: chain.clone(),
            version: tokera::model::PORT_COMMAND_VERSION,
        };

        // Build the rx and tx
        let rx = Box::new(rx);

        // Accept the web connection
        self.accept_internal(
            rx,
            tx,
            hello,
            hello_switch,
            sock_addr,
            None
        ).await
    }
}

struct SwitchFactory
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
for SwitchFactory
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
                let edge_session = impersonate_command(
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

        /*
        - Debugging code
        error!("{}", edge_session);
        use std::ops::Deref;
        error!("{}", master_authority.deref());
        */

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