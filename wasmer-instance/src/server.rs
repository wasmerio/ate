use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use ate::comms::RawWebRoute;
use wasmer_ssh::wasmer_os::environment::Environment;
use wasmer_ssh::wasmer_os::fd::FdMsg;
use error_chain::bail;
#[allow(unused_imports)]
use tokio::sync::mpsc;

use async_trait::async_trait;
use ate::comms::HelloMetadata;
use ate::comms::RawStreamRoute;
use ate::comms::StreamRoute;
use ate::comms::StreamRx;
use ate::comms::StreamReadable;
use ate::comms::MessageProtocolVersion;
use ate::comms::Upstream;
use ate::prelude::*;
use ate_files::repo::Repository;
use ate_files::repo::RepositorySessionFactory;
use wasmer_ssh::wasmer_os::api::System;
use wasmer_ssh::wasmer_os::api::SystemAbiExt;
use wasmer_deploy_cli::model::InstanceHello;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;
use percent_encoding::{percent_decode};
use wasmer_deploy_cli::model::MasterAuthority;
use wasmer_deploy_cli::model::ServiceInstance;
use wasmer_deploy_cli::model::InstanceReply;
use wasmer_deploy_cli::model::INSTANCE_ROOT_ID;
use wasmer_deploy_cli::model::MASTER_AUTHORITY_ID;
#[allow(unused_imports)]
use wasmer_deploy_cli::model::InstanceCall;
use wasmer_ssh::wasmer_os;
use wasmer_os::api::ConsoleRect;
use wasmer_os::fs::UnionFileSystem;
use wasmer_os::bin_factory::*;
use wasmer_os::reactor::Reactor;
use wasmer_os::bus::SubProcessMultiplexer;
use wasmer_os::pipe::pipe_in;
use wasmer_os::pipe::pipe_out;
use wasmer_os::fd::FdFlag;
use wasmer_os::pipe::ReceiverMode;
use wasmer_os::grammar::ast::Redirect;
use wasmer_os::grammar::ast::RedirectionType;
use wasmer_auth::cmd::impersonate_command;
use wasmer_auth::helper::b64_to_session;
use ttl_cache::TtlCache;
use tokio::sync::RwLock;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use http::StatusCode;

use crate::adapter::FileAccessorAdapter;
use crate::session::Session;
use crate::fixed_reader::FixedReader;

#[derive(Clone)]
pub struct SessionBasics {
    pub fs: UnionFileSystem,
    pub bins: BinFactory,
    pub reactor: Arc<RwLock<Reactor>>,
    pub service_instance: DaoMut<ServiceInstance>,
    pub multiplexer: SubProcessMultiplexer
}

pub struct Server
{
    pub system: System,
    pub registry: Arc<Registry>,
    pub repo: Arc<Repository>,
    pub db_url: url::Url,
    pub auth_url: url::Url,
    pub engine: Option<wasmer_os::wasmer::Engine>,
    pub compiler: wasmer_os::eval::Compiler,
    pub compiled_modules: Arc<CachedCompiledModules>,
    pub instance_authority: String,
    pub sessions: RwLock<TtlCache<ChainKey, SessionBasics>>,
    pub ttl: Duration,
}

impl Server
{
    pub async fn new(
        db_url: Url,
        auth_url: Url,
        instance_authority: String,
        token_path: String,
        registry: Arc<Registry>,
        compiler: wasmer_os::eval::Compiler,
        compiled_modules: Arc<CachedCompiledModules>,
        ttl: Duration,
    ) -> Result<Self, Box<dyn std::error::Error>>
    {
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

        let sessions = RwLock::new(TtlCache::new(usize::MAX));
        let engine = compiler.new_engine();

        Ok(Self {
            system: System::default(),
            db_url,
            auth_url,
            registry,
            repo,
            engine,
            compiler,
            compiled_modules,
            instance_authority,
            sessions,
            ttl,
        })
    }

    pub async fn get_or_create_session_basics(&self, key: ChainKey) -> Result<(SessionBasics, bool), CommsError> {
        // Check the cache
        {
            let guard = self.sessions.read().await;
            if let Some(ret) = guard.get(&key) {
                return Ok((ret.clone(), false));
            }
        }

        // Open the instance chain that backs this particular instance
        // (this will reuse accessors across threads and calls)
        let accessor = self.repo.get_accessor(&key, self.instance_authority.as_str()).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;
        let mut fs = wasmer_os::fs::create_root_fs(Some(Box::new(FileAccessorAdapter::new(&accessor))));
        fs.solidify();
        trace!("loaded file file system for {}", key);

        // Load the service instance object
        let _chain = accessor.chain.clone();
        let chain_dio = accessor.dio.clone().as_mut().await;
        trace!("loading service instance with key {}", PrimaryKey::from(INSTANCE_ROOT_ID));
        let service_instance = chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;

        // Enter a write lock and check again
        let mut guard = self.sessions.write().await;
        if let Some(ret) = guard.get(&key) {
            return Ok((ret.clone(), false));
        }

        // Create the bin factory
        let bins = BinFactory::new(self.compiled_modules.clone());
        let reactor = Arc::new(RwLock::new(Reactor::new()));
        let multiplexer = SubProcessMultiplexer::new();
        
        // Build the basics
        let basics = SessionBasics {
            fs,
            bins,
            reactor,
            service_instance,
            multiplexer,
        };

        // Cache and and return it
        let ret = basics.clone();
        guard.insert(key.clone(), basics, self.ttl);
        Ok((ret, true))
    }

    pub async fn new_session(
        &self,
        rx: StreamRx,
        tx: Upstream,
        hello: HelloMetadata,
        hello_instance: InstanceHello,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<Session, CommsError>
    {
        // Get or create the basics that make up a new session
        let key = hello_instance.chain.clone();
        let (basics, first_init) = self.get_or_create_session_basics(key.clone()).await?;

        // Build the session
        let rx = Box::new(rx);
        let ret = Session::new(
            rx,
            Some(tx),
            hello,
            hello_instance,
            sock_addr,
            wire_encryption,
            Arc::new(Mutex::new(ConsoleRect { cols: 80, rows: 25 })),
            self.engine.clone(),
            self.compiler.clone(),
            basics.clone(),
            first_init
        ).await;

        Ok(ret)
    }

    async fn accept_internal(
        &self,
        rx: StreamRx,
        tx: Upstream,
        hello: HelloMetadata,
        hello_instance: InstanceHello,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>
    {
        // Create the session
        let session = self.new_session(
            rx,
            tx,
            hello,
            hello_instance,
            sock_addr,
            wire_encryption
        ).await?;

        // Start the background thread that will process events on the session
        self.system.fork_shared(|| async move {
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
        _rx_proto: StreamProtocol,
        tx: Upstream,
        hello: HelloMetadata,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>
    {
        // Read the instance hello message
        let hello_buf = rx.read().await?;
        let hello_instance: InstanceHello = serde_json::from_slice(&hello_buf[..])?;
        debug!("accept-web-socket: {}", hello_instance);

        // Accept the web connection
        self.accept_internal(
            rx,
            tx,
            hello,
            hello_instance,
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
        // Create the upstream
        let (rx, tx) = MessageProtocolVersion::V3
            .create(Some(rx), Some(tx))
            .split(None);
        let tx = Upstream {
            id: NodeId::generate_client_id(),
            outbox: tx,
            wire_format: SerializationFormat::Json,
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
        let hello = HelloMetadata {
            client_id: NodeId::generate_client_id(),
            server_id,
            path: path.to_string_lossy().to_string(),
            encryption: None,
            wire_format: tx.wire_format,
        };
        let hello_instance = InstanceHello {
            access_token: auth.to_str().unwrap().to_string(),
            chain: chain.clone(),
        };

        // Accept the web connection
        self.accept_internal(
            rx,
            tx,
            hello,
            hello_instance,
            sock_addr,
            None
        ).await
    }
}

#[async_trait]
impl RawWebRoute
for Server
{
    #[allow(unused_variables)]
    async fn accepted_raw_put_request(
        &self,
        uri: http::Uri,
        headers: http::HeaderMap,
        sock_addr: SocketAddr,
        server_id: NodeId,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, (Vec<u8>, StatusCode)>
    {
        // Get the chain and the topic
        let path = std::path::PathBuf::from(uri.path().to_string());
        let (chain, binary, topic) = {
            let mut path_iter = path.iter().map(|a| a.to_string_lossy().to_string());
            path_iter.next();
            path_iter.next();
            let identity = path_iter.next();
            let db = path_iter.next();
            let binary = path_iter.next();
            let topic = path_iter.next();

            if identity.is_none() || db.is_none() || binary.is_none() || topic.is_none() {
                let msg = format!("The URL path is malformed").as_bytes().to_vec();
                return Err((msg, StatusCode::BAD_REQUEST));
            }

            let identity = identity.unwrap();
            let db = db.unwrap();
            let binary = binary.unwrap();
            let topic = topic.unwrap();

            let chain = format!("{}/{}", identity, db);
            (chain, binary, topic)
        };
        let chain = ChainKey::new(chain);
        
        // Get the authorization
        if headers.contains_key(http::header::AUTHORIZATION) == false {
            let msg = format!("Missing the Authorization header").as_bytes().to_vec();
            return Err((msg, StatusCode::UNAUTHORIZED));
        }
        let auth = headers[http::header::AUTHORIZATION].clone();

        // Get and check the data format
        if headers.contains_key(http::header::CONTENT_TYPE) == false {
            let msg = format!("Must supply a content type in the request").as_bytes().to_vec();
            return Err((msg, StatusCode::BAD_REQUEST));
        }
        let format = match headers[http::header::CONTENT_TYPE].to_str().unwrap() {
            "text/xml" |
            "application/xml" |
            "application/xhtml+xml" => SerializationFormat::Xml,
            "application/octet-stream" => SerializationFormat::Raw,
            "application/json" |
            "application/x-javascript" |
            "application/ld+json" |
            "text/javascript" |
            "text/x-javascript" |
            "text/x-json" => SerializationFormat::Json,
            "text/x-yaml" |
            "text/yaml" |
            "text/yml" |
            "application/x-yaml" |
            "application/x-yml" |
            "application/yaml" |
            "application/yml" => SerializationFormat::Yaml,
            a => {
                let msg = format!("Unsupported http content type [{}]", a).as_bytes().to_vec();
                return Err((msg, StatusCode::BAD_REQUEST));
            }
        };

        debug!("accept-raw-put-request: uri: {} (format={})", uri, format);

        // Make a fake hello from the HTTP metadata
        let hello = HelloMetadata {
            client_id: NodeId::generate_client_id(),
            server_id,
            path: path.to_string_lossy().to_string(),
            encryption: None,
            wire_format: SerializationFormat::Json,
        };
        let hello_instance = InstanceHello {
            access_token: auth.to_str().unwrap().to_string(),
            chain: chain.clone(),
        };

        // Get or create the basics that make up a new session
        let key = hello_instance.chain.clone();
        let (basics, first_init) = self.get_or_create_session_basics(key.clone())
            .await
            .map_err(|err| {
                let msg = format!("instance call failed - {}", err).as_bytes().to_vec();
                (msg, StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Create a fixed reader
        let rx: Box<dyn StreamReadable + Send + Sync + Unpin + 'static> = Box::new(FixedReader::new(Vec::new()));
        
        // Build the session
        let mut session = Session::new(
            rx,
            None,
            hello,
            hello_instance,
            sock_addr,
            None,
            Arc::new(Mutex::new(ConsoleRect { cols: 80, rows: 25 })),
            self.engine.clone(),
            self.compiler.clone(),
            basics.clone(),
            first_init
        ).await;

        // Validate we can access this binary
        if session.can_access_binary(binary.as_str(), auth.to_str().unwrap()).await == false {
            let msg = format!("Access Denied (Invalid Token)").as_bytes().to_vec();
            return Err((msg, StatusCode::UNAUTHORIZED));
        }
        
        // Invoke the call
        let (tx_reply, mut rx_reply) = mpsc::channel(1);
        session.call(InstanceCall {
                parent: None,
                handle: fastrand::u64(..),
                format,
                binary,
                topic,
            },
            body,
            tx_reply,
            )
            .await
            .map_err(|err: Box<dyn std::error::Error>| {
                let msg = format!("instance call failed - {}", err).as_bytes().to_vec();
                (msg, StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Read the result and pump it
        loop {
            let invocations = session.invocations.clone();
            tokio::select! {
                reply = rx_reply.recv() => {
                    if let Some(reply) = reply {
                        match reply {
                            InstanceReply::FeedBytes { data, .. } => {
                                return Ok(data);
                            }
                            InstanceReply::Stderr { data } => {
                                trace!("{}", String::from_utf8_lossy(&data[..]));
                            },
                            InstanceReply::Stdout { data } => {
                                trace!("{}", String::from_utf8_lossy(&data[..]));
                            },
                            err => {
                                let msg = format!("Instance call failed - {}", err).as_bytes().to_vec();
                                return Err((msg, StatusCode::BAD_GATEWAY));
                            }
                        }
                    } else {
                        break;
                    }
                },
                _ = invocations => { }
            }
        }
        let msg = format!("Instance call aborted before finishing").as_bytes().to_vec();
        Err((msg, StatusCode::NOT_ACCEPTABLE))
    }

    #[allow(unused_variables)]
    async fn accepted_raw_post_request(
        &self,
        uri: http::Uri,
        headers: http::HeaderMap,
        sock_addr: SocketAddr,
        server_id: NodeId,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, (Vec<u8>, StatusCode)>
    {
        // Get the chain and the binary
        let mut args = Vec::new();
        let mut redirects = Vec::new();
        let path = std::path::PathBuf::from(uri.path().to_string());
        let (chain, binary) = {
            let mut path_iter = path.iter().map(|a| a.to_string_lossy().to_string());
            path_iter.next();
            path_iter.next();
            let identity = path_iter.next();
            let db = path_iter.next();
            let binary = path_iter.next();

            if identity.is_none() || db.is_none() || binary.is_none() {
                let msg = format!("The URL path is malformed").as_bytes().to_vec();
                return Err((msg, StatusCode::BAD_REQUEST));
            }

            let identity = identity.unwrap();
            let db = db.unwrap();
            let binary = binary.unwrap();

            while let Some(arg) = path_iter.next() {
                let arg = percent_decode(arg.as_bytes());
                let arg = arg.decode_utf8_lossy().to_string();

                if let Some((lhr, mut rhs)) = arg.split_once(">") {
                    if let Ok(fd) = i32::from_str(lhr) {
                        let op = if rhs.starts_with(">") {
                            rhs = &rhs[1..];
                            RedirectionType::APPEND
                        } else if rhs.starts_with("|") {
                            rhs = &rhs[1..];
                            RedirectionType::CLOBBER
                        } else if rhs.starts_with("&") {
                            rhs = &rhs[1..];
                            RedirectionType::TOFD
                        } else { RedirectionType::TO };
                        redirects.push(Redirect {
                            fd,
                            op,
                            filename: rhs.to_string(),
                        });
                        continue;
                    }
                }
                args.push(arg);
            }

            let chain = format!("{}/{}", identity, db);
            (chain, binary)
        };
        let chain = ChainKey::new(chain);        
        
        // Get the authorization
        if headers.contains_key(http::header::AUTHORIZATION) == false {
            let msg = format!("Missing the Authorization header").as_bytes().to_vec();
            return Err((msg, StatusCode::UNAUTHORIZED));
        }
        let auth = headers[http::header::AUTHORIZATION].clone();

        // Make a fake hello from the HTTP metadata
        let hello = HelloMetadata {
            client_id: NodeId::generate_client_id(),
            server_id,
            path: path.to_string_lossy().to_string(),
            encryption: None,
            wire_format: SerializationFormat::Json,
        };
        let hello_instance = InstanceHello {
            access_token: auth.to_str().unwrap().to_string(),
            chain: chain.clone(),
        };

        // Get or create the basics that make up a new session
        let key = hello_instance.chain.clone();
        let (basics, first_init) = self.get_or_create_session_basics(key.clone())
            .await
            .map_err(|err| {
                debug!("instance eval failed - {}", err);
                (Vec::new(), StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Build the session
        let rx = Box::new(FixedReader::new(Vec::new()));
        let mut session = Session::new(
            rx,
            None,
            hello,
            hello_instance,
            sock_addr,
            None,
            Arc::new(Mutex::new(ConsoleRect { cols: 80, rows: 25 })),
            self.engine.clone(),
            self.compiler.clone(),
            basics.clone(),
            first_init
        ).await;

        // Validate we can access this binary
        if session.can_access_binary(binary.as_str(), auth.to_str().unwrap()).await == false {
            let msg = format!("Access Denied (Invalid Token)").as_bytes().to_vec();
            return Err((msg, StatusCode::UNAUTHORIZED));
        }

        debug!("accept-raw-post-request: uri: {}", uri);

        // Build an environment from the query string
        let mut env = Environment::default();
        if let Some(query) = uri.query() {
            for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
                env.set_var(&k, v.to_string());
            }
        }

        // Create the stdin pipe
        let (stdin, body_tx) = pipe_in(ReceiverMode::Stream, FdFlag::Stdin(false));
        let _ = body_tx.send(FdMsg::Data { data: body, flag: FdFlag::Stdin(false) }).await;
        let _ = body_tx.send(FdMsg::Data { data: Vec::new(), flag: FdFlag::Stdin(false) }).await;
        drop(body_tx);

        // Create a stdout pipe that will gather the return data
        let (mut stdout, ret_rx) = pipe_out(FdFlag::Stdout(false));
        let (mut stderr, err_rx) = pipe_out(FdFlag::Stdout(false));
        stdout.set_ignore_flush(true);
        stderr.set_ignore_flush(true);

        // Evaluate the binary until its finished
        let exit_code = session.eval(binary, env, args, redirects, stdin, stdout, stderr)
            .await
            .map_err(|err: Box<dyn std::error::Error>| {
                let msg = format!("instance eval failed - {}", err).as_bytes().to_vec();
                (msg, StatusCode::INTERNAL_SERVER_ERROR)
            })?;
        drop(session);

        // Read all the data
        let ret = read_to_end(ret_rx).await;
        debug!("eval returned {} bytes", ret.len());
        
        // Convert the error code to a status code
        match exit_code {
            0 => Ok(ret),
            _ => {
                let err = read_to_end(err_rx).await;
                Err((err, StatusCode::INTERNAL_SERVER_ERROR))
            }
        }
    }
}

async fn read_to_end(mut rx: mpsc::Receiver<FdMsg>) -> Vec<u8>
{
    let mut ret = Vec::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            FdMsg::Data { mut data, .. } => {
                ret.append(&mut data);
            }
            FdMsg::Flush { .. } => { }
        }
    }
    ret
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
        let chain = self.registry.open(&self.db_url, &key, false).await?;
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