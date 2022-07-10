use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use error_chain::bail;
#[allow(unused_imports)]
use tokio::sync::mpsc;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

use async_trait::async_trait;
use ate::comms::HelloMetadata;
use ate::comms::RawStreamRoute;
use ate::comms::StreamRoute;
use ate::comms::StreamRx;
use ate::comms::Upstream;
use ate::comms::MessageProtocolVersion;
use ate::prelude::*;
use ate_files::repo::Repository;
use ate_files::repo::RepositorySessionFactory;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;
use wasmer_deploy::model::MasterAuthority;
use wasmer_deploy::model::MASTER_AUTHORITY_ID;
#[allow(unused_imports)]
use wasmer_deploy::model::InstanceCall;
use wasmer_deploy::model::SwitchHello;
use wasmer_auth::cmd::impersonate_command;
use wasmer_auth::helper::b64_to_session;
use std::sync::RwLock;

use super::factory::*;
use super::session::*;
use super::udp::*;

pub struct Server
{
    pub registry: Arc<Registry>,
    pub repo: Arc<Repository>,
    pub udp: UdpPeerHandle,
    pub db_url: url::Url,
    pub auth_url: url::Url,
    pub instance_authority: String,
    pub factory: Arc<SwitchFactory>,
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
        udp_listen: IpAddr,
        udp_port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>>
    {
        // Build a switch factory that will connect clients to the current switch
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

        let switches = Arc::new(RwLock::new(HashMap::default()));
        let udp = UdpPeer::new(udp_listen, udp_port, switches.clone()).await;

        let factory = Arc::new(
            SwitchFactory::new(repo.clone(), udp.clone(), instance_authority.clone(), switches)
        );

        Ok(Self {
            db_url,
            auth_url,
            registry,
            repo,
            instance_authority,
            udp,
            factory,
        })
    }

    async fn accept_internal(
        &self,
        rx: StreamRx,
        _rx_proto: StreamProtocol,
        tx: Upstream,
        hello: HelloMetadata,
        hello_switch: SwitchHello,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>
    {
        // Get or create the switch
        let key = hello_switch.chain.clone();
        debug!("accept_internal(chain={})", key);
        let (switch, _) = self.factory.get_or_create_switch(key.clone()).await?;

        // Check to make sure the caller has rights to this switch
        if switch.has_access(hello_switch.access_token.as_str()) == false {
            warn!("access denied (id={})", switch.id);
            return Err(CommsErrorKind::Refused.into());
        }

        // Create the port into the switch
        let port = switch.new_port()
            .await
            .map_err(|err| {
                warn!("switch port creation failed - {}", err);
                CommsErrorKind::InternalError(err.to_string())
            })?;
        let mac = port.mac;

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

        info!("switch port established (switch={}, mac={}, peer_addr={})", session.hello_switch.chain, mac, sock_addr);
        
        // Start the background thread that will process events on the session
        tokio::task::spawn(async move {
            if let Err(err) = session.run().await {
                warn!("instance session failed: {}", err);
            }
            info!("switch port closed (switch={}, mac={}, addr={})", key, mac, sock_addr);
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
        rx_proto: StreamProtocol,
        tx: Upstream,
        hello: HelloMetadata,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>
    {
        // Read the instance hello message
        let hello_buf = rx.read().await?;
        let hello_switch: SwitchHello = serde_json::from_slice(&hello_buf[..])?;
        debug!("accept-web-socket: {}", hello_switch);

        // Accept the web connection
        self.accept_internal(
            rx,
            rx_proto,
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
        rx: Box<dyn AsyncRead + Send + Sync + Unpin + 'static>,
        tx: Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>,
        uri: http::Uri,
        headers: http::HeaderMap,
        sock_addr: SocketAddr,
        server_id: NodeId,
    ) -> Result<(), CommsError>
    {
        // Check if its https or not
        let https = uri.scheme_str() == Some("https") || uri.scheme_str() == Some("wss");
        let stream_proto = if https {
            StreamProtocol::SecureWebSocket
        } else {
            StreamProtocol::WebSocket
        };

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
        let client_id = NodeId::generate_client_id();
        let hello = HelloMetadata {
            client_id,
            server_id,
            path: path.to_string_lossy().to_string(),
            encryption: None,
            wire_format: SerializationFormat::Bincode,
        };
        let hello_switch = SwitchHello {
            chain: chain.clone(),
            access_token: auth.to_str().unwrap().to_string(),
            version: wasmer::model::PORT_COMMAND_VERSION,
        };

        // Build the rx and tx
        let (rx, tx) = MessageProtocolVersion::V3
            .create(Some(rx), Some(tx))
            .split(None);
        let tx = Upstream {
            id: client_id,
            outbox: tx,
            wire_format: SerializationFormat::Bincode
        };

        // Accept the web connection
        self.accept_internal(
            rx,
            stream_proto,
            tx,
            hello,
            hello_switch,
            sock_addr,
            None
        ).await
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
                    warn!("token is missing - {}", self.token_path);
                    let err: wasmer_auth::error::GatherError = wasmer_auth::error::GatherErrorKind::NoMasterKey.into();
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
            let err: wasmer_auth::error::GatherError = wasmer_auth::error::GatherErrorKind::NoMasterKey.into();
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