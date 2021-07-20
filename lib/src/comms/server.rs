#![allow(unused_imports)]
use log::{info, warn, debug, error};
use tokio::{net::{TcpListener}};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use crate::error::*;
use tokio::time::Duration;
use std::sync::Arc;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;
use crate::crypto::KeySize;
use crate::spec::*;
use std::{marker::PhantomData};

use std::convert::Infallible;
#[cfg(feature="http_ws")]
use hyper::{header, upgrade, StatusCode, Body, Request, Response, Server, server::conn::AddrStream};
#[cfg(feature="http_ws")]
use hyper::service::{make_service_fn, service_fn};
#[cfg(feature="ws")]
use tokio_tungstenite::WebSocketStream;
#[cfg(feature="ws")]
use tokio_tungstenite::tungstenite::{handshake, Error};

use super::BroadcastContext;
use super::BroadcastPacketData;
use super::PacketWithContext;
use super::conf::*;
use super::rx_tx::*;
use super::helper::*;
use super::key_exchange;
use super::Stream;
use super::StreamProtocol;

pub(crate) async fn listen<M, C>(conf: &NodeConfig<M>) -> (NodeTx<C>, NodeRx<M, C>)
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone + 'static,
      C: Send + Sync + BroadcastContext + Default + 'static
{
    // Setup the communication pipes for the server
    let (inbox_tx, inbox_rx) = mpsc::channel(conf.buffer_size);
    let (downcast_tx, _) = broadcast::channel(conf.buffer_size);
    let downcast_tx = Arc::new(downcast_tx);

    // Create the node state and initialize it
    let state = Arc::new(StdMutex::new(NodeState {
        connected: 0,
    }));

    // Create all the listeners
    for target in conf.listen_on.iter() {
        listen_on::<M, C>(
            target.clone(), 
            inbox_tx.clone(), 
            Arc::clone(&downcast_tx),
            conf.buffer_size,
            Arc::clone(&state),
            conf.path_filter.clone(),
            conf.wire_protocol,
            conf.wire_format,
            conf.wire_encryption,
        ).await;
    }

    // Return the node transmit and receive handlers
    (
        NodeTx {
            direction: TxDirection::Downcast(downcast_tx),
            state: Arc::clone(&state),
            wire_protocol: conf.wire_protocol,
            wire_format: conf.wire_format,
            _marker: PhantomData
        },
        NodeRx {
            rx: inbox_rx,
            state: state,
            _marker: PhantomData
        }
    )
}

pub(super) async fn listen_on<M, C>(
    addr: SocketAddr,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    outbox: Arc<broadcast::Sender<BroadcastPacketData>>,
    buffer_size: usize,
    state: Arc<StdMutex<NodeState>>,
    path_filter: String,
    wire_protocol: StreamProtocol,
    wire_format: SerializationFormat,
    wire_encryption: Option<KeySize>,
)
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
C: Send + Sync + BroadcastContext + Default + 'static,
{
    match wire_protocol.is_http()
    {
        false => listen_on_with_tcp(addr, inbox, outbox, buffer_size, state, wire_protocol, wire_format, wire_encryption).await,
        true => listen_on_with_http(addr, inbox, outbox, buffer_size, state, path_filter, wire_protocol, wire_format, wire_encryption).await,
    }
}

pub(super) async fn listen_on_with_tcp<M, C>(
                            addr: SocketAddr,
                            inbox: mpsc::Sender<PacketWithContext<M, C>>,
                            outbox: Arc<broadcast::Sender<BroadcastPacketData>>,
                            buffer_size: usize,
                            state: Arc<StdMutex<NodeState>>,
                            wire_protocol: StreamProtocol,
                            wire_format: SerializationFormat,
                            wire_encryption: Option<KeySize>,
                        )
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + BroadcastContext + Default + 'static,
{
    let listener = TcpListener::bind(addr.clone()).await
        .expect(&format!("Failed to bind listener to address ({})", addr.clone()));

    let mut exp_backoff = Duration::from_millis(100);
    tokio::task::spawn(async move {
        loop {
            let (stream, sock_addr) = match listener.accept().await {
                Ok(a) => a,
                Err(err) => {
                    eprintln!("tcp-listener - {}", err.to_string());
                    tokio::time::sleep(exp_backoff).await;
                    exp_backoff *= 2;
                    if exp_backoff > Duration::from_secs(10) { exp_backoff = Duration::from_secs(10); }
                    continue;
                }
            };
            exp_backoff = Duration::from_millis(100);
            info!("connection-from: {}", sock_addr.to_string());
            
            setup_tcp_stream(&stream).unwrap();

            let stream = Stream::Tcp(stream);

            let inbox = inbox.clone();
            let outbox = Arc::clone(&outbox);
            let state = Arc::clone(&state);

            match accept_tcp_connect
            (
                stream,
                sock_addr,
                inbox,
                outbox,
                buffer_size,
                state,
                wire_protocol,
                wire_format,
                wire_encryption,
            ).await {
                Ok(a) => a,
                Err(CommsError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => { }
                Err(err) => {
                    warn!("connection-failed: {}", err.to_string());
                    continue;
                }
            };
        }
    });
}

pub(super) async fn accept_tcp_connect<M, C>(
    stream: Stream,
    sock_addr: SocketAddr,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    outbox: Arc<broadcast::Sender<BroadcastPacketData>>,
    buffer_size: usize,
    state: Arc<StdMutex<NodeState>>,
    wire_protocol: StreamProtocol,
    wire_format: SerializationFormat,
    wire_encryption: Option<KeySize>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + BroadcastContext + Default + 'static,
{
    let stream = stream.upgrade_server(wire_protocol).await?;

    {
        // Increase the connection count
        let mut guard = state.lock();
        guard.connected = guard.connected + 1;
    }

    // Split the stream
    let (mut stream_rx, mut stream_tx) = stream.split();

    // Say hello
    let wire_encryption = super::hello::mesh_hello_exchange_receiver
    (
        &mut stream_rx,
        &mut stream_tx,
        wire_encryption,
        wire_format
    ).await?;

    // If we are using wire encryption then exchange secrets
    let ek = match wire_encryption {
        Some(key_size) => Some(
            key_exchange::mesh_key_exchange_receiver
            (
                &mut stream_rx,
                &mut stream_tx,
                key_size
            ).await?
        ),
        None => None,
    };
    let ek1 = ek.clone();
    let ek2 = ek.clone();

    let context = Arc::new(C::default());
    let sender = fastrand::u64(..);

    let (terminate_tx, _) = tokio::sync::broadcast::channel::<bool>(1);
    let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
    let reply_tx1 = reply_tx.clone();
    let reply_tx2 = reply_tx.clone();

    let worker_context = Arc::clone(&context);
    let worker_state = Arc::clone(&state);
    let worker_inbox = inbox.clone();
    let worker_terminate_tx = terminate_tx.clone();
    let worker_terminate_rx = terminate_tx.subscribe();
    tokio::spawn(async move {
        match process_inbox::<M, C>
        (
            stream_rx,
            reply_tx1,
            worker_inbox,
            sender,
            worker_context,
            wire_format,
            ek1,
            worker_terminate_rx
        ).await {
            Ok(_) => {},
            Err(CommsError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => { }
            Err(err) => warn!("connection-failed (inbox): {}", err.to_string())
        };
        info!("disconnected: {}", sock_addr.to_string());
        let _ = worker_terminate_tx.send(true);

        // Decrease the connection state
        let mut guard = worker_state.lock();
        guard.connected = guard.connected - 1;
    });

    let worker_terminate_tx = terminate_tx.clone();
    let worker_terminate_rx = terminate_tx.subscribe();
    tokio::spawn(async move {
        match process_outbox::<M>
        (
            stream_tx,
            reply_rx,
            sender,
            ek2,
            worker_terminate_rx
        ).await {
            Ok(_) => {},
            Err(CommsError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => { }
            Err(err) => warn!("connection-failed (outbox): {}", err.to_string())
        };
        let _ = worker_terminate_tx.send(true);
    });

    let worker_context = Arc::clone(&context);
    let worker_outbox = outbox.subscribe();
    let worker_terminate_tx = terminate_tx.clone();
    let worker_terminate_rx = terminate_tx.subscribe();
    tokio::spawn(async move {
        match process_downcast::<M, C>
        (
            reply_tx2,
            worker_outbox,
            sender,
            worker_context,
            worker_terminate_rx
        ).await {
            Ok(_) => {},
            Err(CommsError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => { }
            Err(err) => warn!("connection-failed (downcast): {}", err.to_string())
        };
        let _ = worker_terminate_tx.send(true);
    });

    Ok(())
}

pub(super) async fn handle_http_request<M, C>(
                        mut request: Request<Body>,
                        remote_addr: SocketAddr,
                        inbox: mpsc::Sender<PacketWithContext<M, C>>,
                        outbox: Arc<broadcast::Sender<BroadcastPacketData>>,
                        buffer_size: usize,
                        state: Arc<StdMutex<NodeState>>,
                        path_filter: String,
                        wire_protocol: StreamProtocol,
                        wire_format: SerializationFormat,
                        wire_encryption: Option<KeySize>) -> Result<Response<Body>, Infallible>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + BroadcastContext + Default + 'static,
{
    info!("handle_http_request: {}", request.uri().to_string());
    
    let path_filter = path_filter.as_str();
    match (request.uri().path(), request.headers().contains_key(header::UPGRADE)) {
        (a, true) if a == path_filter => {
        
            let response = 
            match handshake::server::create_response_with_body(&request, || Body::empty()) {
                Ok(response) => {
                    tokio::spawn(async move {
                        match upgrade::on(&mut request).await {
                            Ok(upgraded) => {
                                let ws_stream = WebSocketStream::from_raw_socket(
                                    upgraded,
                                    tokio_tungstenite::tungstenite::protocol::Role::Server,
                                    None,
                                ).await;

                                let stream = Stream::WebSocketUpgraded(ws_stream, wire_protocol);
                                let sock_addr = remote_addr;

                                match accept_tcp_connect
                                (
                                    stream,
                                    sock_addr,
                                    inbox,
                                    outbox,
                                    buffer_size,
                                    state,
                                    wire_protocol,
                                    wire_format,
                                    wire_encryption,
                                ).await {
                                    Ok(a) => a,
                                    Err(CommsError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => { }
                                    Err(err) => {
                                        warn!("connection-failed: {}", err.to_string());
                                    }
                                };
                            },
                            Err(e) =>
                                error!("error when trying to upgrade connection \
                                        from address {} to websocket connection. \
                                        Error is: {}", remote_addr, e),
                        }
                    });
                    response
                },
                Err(error) => {
                    error!("Failed to create websocket response \
                                to request from address {}. \
                                Error is: {}", remote_addr, error);
                    let mut res = Response::new(Body::from(format!("Failed to create websocket: {}", error)));
                    *res.status_mut() = StatusCode::BAD_REQUEST;
                    return Ok(res);
                }
            };
        
            Ok::<_, Infallible>(response)
        },
        (a, false) if a == path_filter => {
            Ok(Response::new(Body::from(format!("Only connections over \
                                                websocket clients are supported
                                                for this ATE chain-of-trust.\n"))))
        },
        (_, false) => {
            Ok(Response::new(Body::from(format!("The accessed URL is not currently \
                                                hosting an ATE chain-of-trust and
                                                this server is only accepting
                                                websocket clients.\n"))))
        }
        (_, true) => {
            Ok(Response::new(Body::from(format!("The accessed URL is not currently \
                                                hosting an ATE chain-of-trust.\n"))))
        }
    }
}

pub(super) async fn listen_on_with_http<M, C>(addr: SocketAddr,
                           inbox: mpsc::Sender<PacketWithContext<M, C>>,
                           outbox: Arc<broadcast::Sender<BroadcastPacketData>>,
                           buffer_size: usize,
                           state: Arc<StdMutex<NodeState>>,
                           path_filter: String,
                           wire_protocol: StreamProtocol,
                           wire_format: SerializationFormat,
                           wire_encryption: Option<KeySize>,
                        )
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + BroadcastContext + Default + 'static,
{
    let make_svc = {
        make_service_fn(move |conn: & AddrStream| {
            let remote_addr = conn.remote_addr();
            let inbox = inbox.clone();
            let outbox = Arc::clone(&outbox);
            let state = Arc::clone(&state);
            let path_filter = path_filter.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |request: Request<Body>|
                    {
                        let path_filter = path_filter.clone();
                        let inbox = inbox.clone();
                        let outbox = Arc::clone(&outbox);
                        let state = Arc::clone(&state);
                        handle_http_request::<M, C>(
                            request,
                            remote_addr,
                            inbox,
                            outbox,
                            buffer_size,
                            state,
                            path_filter,
                            wire_protocol,
                            wire_format,
                            wire_encryption
                        )
                    }
                ))
            }
        })
    };

    let listener = Server::bind(&addr).serve(make_svc);

    tokio::task::spawn(async move {
        if let Err(err) = listener.await {
            error!("http-listener failed: {}", err);
        }
    });
}