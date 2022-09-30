use std::ops::DerefMut;
use std::net::SocketAddr;
#[cfg(not(feature = "enable_dns"))]
use std::net::ToSocketAddrs;
use std::result::Result;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use error_chain::bail;
use fxhash::FxHashMap;
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "enable_full")]
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_futures::{Instrument, WithSubscriber};
use ate_comms::MessageProtocolApi;

#[allow(unused_imports)]
use crate::conf::*;
use crate::crypto::*;
use crate::engine::TaskEngine;
use crate::spec::*;
use crate::{comms::NodeId, error::*};

use super::hello;
use super::helper::*;
use super::key_exchange;
use super::metrics::*;
use super::rx_tx::*;
use super::throttle::*;
use super::CertificateValidation;
use super::{conf::*, hello::HelloMetadata};
#[allow(unused_imports)]
use {
    super::StreamProtocol, super::StreamRx, super::StreamTx,
};

pub(crate) async fn connect<M, C>(
    conf: &MeshConfig,
    hello_path: String,
    node_id: NodeId,
    inbox: impl InboxProcessor<M, C> + 'static,
    metrics: Arc<StdMutex<Metrics>>,
    throttle: Arc<StdMutex<Throttle>>,
    exit: broadcast::Receiver<()>,
) -> Result<Tx, CommsError>
where
    M: Send + Sync + Serialize + DeserializeOwned + Default + Clone + 'static,
    C: Send + Sync + Default + 'static,
{
    // Create all the outbound connections
    if let Some(target) = &conf.connect_to {
        // Perform the connect operation
        let inbox = Box::new(inbox);
        let upstream = mesh_connect_to::<M, C>(
            target.clone(),
            hello_path.clone(),
            node_id,
            conf.cfg_mesh.domain_name.clone(),
            inbox,
            conf.cfg_mesh.wire_protocol,
            conf.cfg_mesh.wire_encryption,
            conf.cfg_mesh.connect_timeout,
            conf.cfg_mesh.fail_fast,
            conf.cfg_mesh.certificate_validation.clone(),
            Arc::clone(&metrics),
            Arc::clone(&throttle),
            exit,
        )
        .await?;

        // Return the mesh
        Ok(Tx {
            direction: TxDirection::Upcast(upstream),
            hello_path: hello_path.clone(),
            wire_format: conf.cfg_mesh.wire_format,
            relay: None,
            metrics: Arc::clone(&metrics),
            throttle: Arc::clone(&throttle),
            exit_dependencies: Vec::new(),
        })
    } else {
        bail!(CommsErrorKind::NoAddress);
    }
}

pub(super) async fn mesh_connect_to<M, C>(
    addr: MeshConnectAddr,
    hello_path: String,
    node_id: NodeId,
    domain: String,
    inbox: Box<dyn InboxProcessor<M, C>>,
    wire_protocol: StreamProtocol,
    wire_encryption: Option<KeySize>,
    timeout: Duration,
    fail_fast: bool,
    validation: CertificateValidation,
    metrics: Arc<StdMutex<super::metrics::Metrics>>,
    throttle: Arc<StdMutex<super::throttle::Throttle>>,
    exit: broadcast::Receiver<()>,
) -> Result<Upstream, CommsError>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
    C: Send + Sync + Default + 'static,
{
    // Make the connection
    trace!("prepare connect (path={})", hello_path);
    let worker_connect = mesh_connect_prepare(
        addr.clone(),
        hello_path,
        node_id,
        domain,
        wire_protocol,
        wire_encryption,
        fail_fast,
    );
    let mut worker_connect =
        crate::engine::timeout(timeout, worker_connect).await??;
    let wire_format = worker_connect.hello_metadata.wire_format;
    let server_id = worker_connect.hello_metadata.server_id;

    // If we are using wire encryption then exchange secrets
    let ek = match wire_encryption {
        Some(key_size) => Some(
            key_exchange::mesh_key_exchange_sender(
                worker_connect.proto.deref_mut(),
                key_size,
                validation,
            )
            .await?,
        ),
        None => None,
    };

    // Split the stream
    let (rx, tx) = worker_connect.proto.split(ek);

    // background thread - connects and then runs inbox and outbox threads
    // if the upstream object signals a termination event it will exit
    trace!("spawning connect worker");
    TaskEngine::spawn(mesh_connect_worker::<M, C>(
        rx,
        wire_protocol,
        wire_format,
        addr,
        ek,
        node_id,
        server_id,
        inbox,
        metrics,
        throttle,
        exit,
    ));

    trace!("building upstream with tx channel");
    Ok(Upstream {
        id: node_id,
        outbox: tx,
        wire_format,
    })
}

struct MeshConnectContext {
    #[allow(dead_code)]
    addr: MeshConnectAddr,
    proto: Box<dyn MessageProtocolApi + Send + Sync + 'static>,
    hello_metadata: HelloMetadata,
}

#[allow(unused_variables)]
async fn mesh_connect_prepare(
    addr: MeshConnectAddr,
    hello_path: String,
    node_id: NodeId,
    domain: String,
    wire_protocol: StreamProtocol,
    wire_encryption: Option<KeySize>,
    #[allow(unused_variables)] fail_fast: bool,
) -> Result<MeshConnectContext, CommsError> {
    async move {
        #[allow(unused_mut)]
        let mut exp_backoff = Duration::from_millis(100);
        loop {
            // If we have a factory then use it
            #[allow(unused_mut)]
            let mut stream = {
                let mut factory = crate::mesh::GLOBAL_COMM_FACTORY.lock().await;
                if let Some(factory) = factory.as_mut() {
                    let create_client = Arc::clone(&factory);
                    drop(factory);
                    create_client(addr.clone()).await
                } else {
                    None
                }
            };

            // If no stream yet exists then create one
            #[cfg(feature = "enable_full")]
            if stream.is_none() {
                stream = {
                    #[cfg(not(feature = "enable_dns"))]
                    let addr = {
                        match format!("{}:{}", addr.host, addr.port)
                            .to_socket_addrs()?
                            .next()
                        {
                            Some(a) => a,
                            None => {
                                bail!(CommsErrorKind::InvalidDomainName);
                            }
                        }
                    };

                    let stream = match TcpStream::connect(addr.clone()).await {
                        Err(err)
                            if match err.kind() {
                                std::io::ErrorKind::ConnectionRefused => {
                                    if fail_fast {
                                        bail!(CommsErrorKind::Refused);
                                    }
                                    true
                                }
                                std::io::ErrorKind::ConnectionReset => true,
                                std::io::ErrorKind::ConnectionAborted => true,
                                _ => false,
                            } =>
                        {
                            debug!(
                                "connect failed: reason={}, backoff={}s",
                                err,
                                exp_backoff.as_secs_f32()
                            );
                            crate::engine::sleep(exp_backoff).await;
                            exp_backoff *= 2;
                            if exp_backoff > Duration::from_secs(10) {
                                exp_backoff = Duration::from_secs(10);
                            }
                            continue;
                        }
                        a => a?,
                    };

                    // Upgrade and split
                    let (rx, tx) = wire_protocol.upgrade_client_and_split(stream).await?;
                    Some((rx, tx))
                };

                #[cfg(all(feature = "enable_web_sys", not(feature = "enable_full")))]
                bail!(CommsErrorKind::InternalError(
                    "Web based clients require a GLOBAL_COMM_FACTORY".to_string()
                ));
            }

            let stream = match stream {
                Some(a) => a,
                None => {
                    bail!(CommsErrorKind::InternalError(
                        "Failed to create a client stream".to_string()
                    ));
                }
            };

            // Build the stream
            trace!("splitting stream into rx/tx");
            let (stream_rx,
                 stream_tx) = stream;

            // Say hello
            let (proto, hello_metadata) = hello::mesh_hello_exchange_sender(
                stream_rx,
                stream_tx,
                node_id,
                hello_path.clone(),
                domain.clone(),
                wire_encryption,
            )
            .await?;

            // Return the result
            return Ok(
                MeshConnectContext {
                    addr,
                    proto,
                    hello_metadata,
                }
            );
        }
    }
    .instrument(tracing::info_span!("connect"))
    .await
}

async fn mesh_connect_worker<M, C>(
    rx: StreamRx,
    rx_proto: StreamProtocol,
    wire_format: SerializationFormat,
    sock_addr: MeshConnectAddr,
    wire_encryption: Option<EncryptKey>,
    node_id: NodeId,
    peer_id: NodeId,
    inbox: Box<dyn InboxProcessor<M, C>>,
    metrics: Arc<StdMutex<super::metrics::Metrics>>,
    throttle: Arc<StdMutex<super::throttle::Throttle>>,
    exit: broadcast::Receiver<()>,
) where
    M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
    C: Send + Sync + Default + 'static,
{
    let span = span!(
        Level::DEBUG,
        "client",
        id = node_id.to_short_string().as_str(),
        peer = peer_id.to_short_string().as_str()
    );

    let context = Arc::new(C::default());
    match process_inbox::<M, C>(
        rx,
        rx_proto,
        inbox,
        metrics,
        throttle,
        node_id,
        peer_id,
        sock_addr.clone(),
        context,
        wire_format,
        wire_encryption,
        exit,
    )
    .instrument(span.clone())
    .await
    {
        Ok(_) => {}
        Err(CommsError(CommsErrorKind::IO(err), _))
            if match err.kind() {
                std::io::ErrorKind::BrokenPipe => true,
                std::io::ErrorKind::UnexpectedEof => true,
                std::io::ErrorKind::ConnectionReset => true,
                std::io::ErrorKind::ConnectionAborted => true,
                _ => false,
            } => {}
        Err(err) => {
            warn!("connection-failed: {}", err.to_string());
        }
    };

    let _span = span.enter();

    //#[cfg(feature = "enable_verbose")]
    debug!("disconnected-inbox: node-id={} addr={}", node_id.to_short_string().as_str(), sock_addr.to_string());
}
