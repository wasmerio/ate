#![allow(unused_imports)]
use log::{info, warn, debug};
use fxhash::FxHashMap;
#[cfg(feature="enable_tcp")]
use tokio::{net::{TcpStream}};
use tokio::time::Duration;
use std::sync::Arc;
use send_wrapper::SendWrapper;
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;
#[cfg(all(feature="enable_tcp", not(feature="enable_dns")))]
use std::net::ToSocketAddrs;
use std::result::Result;

#[cfg(feature="enable_web")]
#[cfg(feature="enable_ws")]
use
{
    ws_stream_wasm        :: { *                                    } ,
};

use crate::error::*;
use crate::crypto::*;
use crate::spec::*;
#[allow(unused_imports)]
use crate::conf::*;
use crate::engine::TaskEngine;

use super::conf::*;
use super::rx_tx::*;
use super::helper::*;
use super::hello;
use super::key_exchange;
#[allow(unused_imports)]
use {
    super::Stream,
    super::StreamRx,
    super::StreamTx,
    super::StreamTxChannel,
    super::StreamProtocol
};

pub(crate) async fn connect<M, C>
(
    conf: &MeshConfig,
    hello_path: String,
    inbox: impl InboxProcessor<M, C> + 'static
)
-> Result<Tx, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone + 'static,
      C: Send + Sync + Default + 'static,
{
    // Create all the outbound connections
    if let Some(target) = &conf.connect_to
    {
        let inbox = Box::new(inbox);
        let upstream = mesh_connect_to::<M, C>(
            target.clone(), 
            hello_path.clone(),
            conf.cfg_mesh.domain_name.clone(),
            inbox,
            conf.cfg_mesh.wire_protocol,
            conf.cfg_mesh.wire_encryption,
            conf.cfg_mesh.connect_timeout,
            conf.cfg_mesh.fail_fast,
        ).await?;
        
        // Return the mesh
        Ok(
            Tx {
                direction: TxDirection::Upcast(upstream),
                hello_path: hello_path.clone(),
                wire_format: conf.cfg_mesh.wire_format,
            },
        )
    }
    else
    {
        return Err(CommsError::NoAddress);
    }
}

#[cfg(feature="enable_dns")]
type MeshConnectAddr = SocketAddr;
#[cfg(not(feature="enable_dns"))]
type MeshConnectAddr = crate::conf::MeshAddress;

pub(super) async fn mesh_connect_to<M, C>
(
    addr: MeshConnectAddr,
    hello_path: String,
    domain: String,
    inbox: Box<dyn InboxProcessor<M, C>>,
    wire_protocol: StreamProtocol,
    wire_encryption: Option<KeySize>,
    timeout: Duration,
    fail_fast: bool,
)
-> Result<Upstream, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static,
{
    // Make the connection
    let sender = fastrand::u64(..);    
    let worker_connect = mesh_connect_prepare
    (
        addr.clone(),
        hello_path,
        domain,
        sender,
        wire_protocol,
        wire_encryption,
        fail_fast,
    );
    let (mut worker_connect, mut stream_tx) = tokio::time::timeout(timeout, worker_connect).await??;
    let wire_format = worker_connect.wire_format;

    // If we are using wire encryption then exchange secrets
    let ek = match wire_encryption {
        Some(key_size) => Some(key_exchange::mesh_key_exchange_sender(&mut worker_connect.stream_rx, &mut stream_tx, key_size).await?),
        None => None,
    };

    // background thread - connects and then runs inbox and outbox threads
    // if the upstream object signals a termination event it will exit
    TaskEngine::spawn(
        async move {
            mesh_connect_worker::<M, C>(worker_connect, addr, ek, inbox).await
        }
    );

    let stream_tx = StreamTxChannel::new(stream_tx, ek);
    Ok(Upstream {
        id: sender,
        outbox: stream_tx,
        wire_format,
    })
}

struct MeshConnectContext
{
    addr: MeshConnectAddr,
    sender: u64,
    stream_rx: StreamRx,
    wire_format: SerializationFormat,
}

async fn mesh_connect_prepare
(
    
    addr: MeshConnectAddr,
    hello_path: String,
    domain: String,
    sender: u64,
    wire_protocol: StreamProtocol,
    wire_encryption: Option<KeySize>,
    #[allow(unused_variables)]
    fail_fast: bool,
)
-> Result<(MeshConnectContext, StreamTx), CommsError>
{
    let mut exp_backoff = Duration::from_millis(100);
    loop {
        #[cfg(all(feature="enable_tcp", not(feature="enable_dns")))]
        let addr = {
            match format!("{}:{}", addr.host, addr.port)
                .to_socket_addrs()?
                .next()
            {
                Some(a) => a,
                None => {
                    return Err(CommsError::InvalidDomainName);
                }
            }
        };

        #[cfg(feature="enable_tcp")]
        let stream = match TcpStream::connect(addr.clone()).await {
            Err(err) if match err.kind() {
                std::io::ErrorKind::ConnectionRefused => {
                    if fail_fast {
                        return Err(CommsError::Refused);
                    }
                    true
                },
                std::io::ErrorKind::ConnectionReset => true,
                std::io::ErrorKind::ConnectionAborted => true,
                _ => false   
            } => {
                debug!("connect failed: reason={}, backoff={}s", err, exp_backoff.as_secs_f32());
                tokio::time::sleep(exp_backoff).await;
                exp_backoff *= 2;
                if exp_backoff > Duration::from_secs(10) { exp_backoff = Duration::from_secs(10); }
                continue;
            },
            a => a?,
        };
         
        #[cfg(feature="enable_tcp")]
        let stream = {
            // Setup the TCP stream
            setup_tcp_stream(&stream)?;

            // Convert the TCP stream into the right protocol
            let stream = Stream::Tcp(stream);
            let stream = stream.upgrade_client(wire_protocol).await?;
            stream
        };

        // Connect to the websocket using the WASM binding (browser connection)
        #[cfg(feature="enable_web")]
        #[cfg(not(feature="enable_tcp"))]
        let stream = {
            let url = wire_protocol.make_url(addr.host.clone(), addr.port, hello_path.clone())?.to_string();
            
            let connect = SendWrapper::new(WsMeta::connect( url, None ));
            let (_, wsio) = match connect.await {
                Ok(a) => a,
                Err(WsErr::ConnectionFailed{ event }) => {
                    debug!("connect failed: reason={}, backoff={}s", event.reason, exp_backoff.as_secs_f32());
                    tokio::time::sleep(exp_backoff).await;
                    exp_backoff *= 2;
                    if exp_backoff > Duration::from_secs(10) { exp_backoff = Duration::from_secs(10); }
                    continue;
                },
                a => a?,
            };

            let stream = SendWrapper::new(wsio.into_io());
            Stream::WebSocket(stream, wire_protocol)
        };

        // Build the stream
        let (mut stream_rx, mut stream_tx) = stream.split();

        // Say hello
        let hello_metadata = hello::mesh_hello_exchange_sender(&mut stream_rx, &mut stream_tx, hello_path.clone(), domain.clone(), wire_encryption).await?;
        let wire_format = hello_metadata.wire_format;

        // Return the result
        return Ok((MeshConnectContext {
            addr,
            sender,
            stream_rx,
            wire_format,
        }, stream_tx));
    }
}

async fn mesh_connect_worker<M, C>
(
    connect: MeshConnectContext,
    sock_addr: MeshConnectAddr,
    wire_encryption: Option<EncryptKey>,
    inbox: Box<dyn InboxProcessor<M, C>>
)
-> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static,
{
    let context = Arc::new(C::default());
    match process_inbox::<M, C>
    (
        connect.stream_rx,
        inbox,
        connect.sender,
        sock_addr,
        context,
        connect.wire_format,
        wire_encryption
    ).await {
        Ok(_) => { },
        Err(CommsError::IO(err)) if match err.kind() {
            std::io::ErrorKind::UnexpectedEof => true,
            std::io::ErrorKind::ConnectionReset => true,
            std::io::ErrorKind::ConnectionAborted => true,
            _ => false,
        } => { },
        Err(err) => {
            warn!("connection-failed: {}", err.to_string());
        },
    };
    //#[cfg(feature = "enable_verbose")]
    debug!("disconnected-inbox: {}", connect.addr.to_string());
    Err(CommsError::Disconnected)
}