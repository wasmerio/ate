use wasm_bus_ws::ws::SocketBuilder;
use wasm_bus_ws::ws::RecvHalf;
use wasm_bus_ws::ws::SendHalf;
use std::io;
use std::time::Duration;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::BTreeMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::model::IpVersion;
use crate::model::IpProtocol;
use crate::model::PortCommand;
use crate::model::PortResponse;
use crate::model::SocketHandle;

use super::evt::*;
use super::socket::*;

const MAX_MPSC: usize = std::usize::MAX >> 3;

pub struct SocketState
{
    recv: mpsc::Sender<EventRecv>,
    recv_from: mpsc::Sender<EventRecvFrom>,
    error: mpsc::Sender<EventError>,
    accept: mpsc::Sender<EventAccept>,
    deconfigure: mpsc::Sender<EventDhcpDeconfigured>,
    configure: mpsc::Sender<EventDhcpConfigured>,
}

pub struct Port
{
    tx: SendHalf,
    rx: Arc<Mutex<RecvHalf>>,
    sockets: Arc<Mutex<BTreeMap<i32, SocketState>>>,
}

impl Port
{
    pub async fn new(url: url::Url) -> io::Result<Port> {
        let builder = SocketBuilder::new(url);
        let ws = builder.open().await?;
        let (tx, rx) = ws.split();
        Ok(Port {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            sockets: Default::default(),
        })
    }

    async fn new_socket(&self) -> Socket {
        let mut sockets = self.sockets.lock().await;
        let handle = sockets.iter()
            .rev()
            .next()
            .map(|e| e.0.clone() + 1)
            .unwrap_or_else(|| 1i32);
        
        let (tx_recv, rx_recv) = mpsc::channel(MAX_MPSC);
        let (tx_recv_from, rx_recv_from) = mpsc::channel(MAX_MPSC);
        let (tx_error, rx_error) = mpsc::channel(MAX_MPSC);
        let (tx_accept, rx_accept) = mpsc::channel(MAX_MPSC);
        let (tx_deconfigure, rx_deconfigure) = mpsc::channel(MAX_MPSC);
        let (tx_configure, rx_configure) = mpsc::channel(MAX_MPSC);

        sockets.insert(handle, SocketState{
            recv: tx_recv,
            recv_from: tx_recv_from,
            error: tx_error,
            accept: tx_accept,
            deconfigure: tx_deconfigure,
            configure: tx_configure,
        });

        let handle = SocketHandle(handle);
        Socket {
            handle,
            tx: self.tx.clone(),
            recv: rx_recv,
            recv_from: rx_recv_from,
            error: rx_error,
            accept: rx_accept,
            deconfigure: rx_deconfigure,
            configure: rx_configure,
        }
    }

    pub async fn bind_raw(&self, ip_version: IpVersion, ip_protocol: IpProtocol) -> io::Result<Socket> {
        let socket = self.new_socket().await;

        socket.tx(PortCommand::BindRaw {
            handle: socket.handle,
            ip_version,
            ip_protocol,
        }).await?;

        Ok(socket)
    }

    pub async fn bind_udp(&self, local_addr: SocketAddr) -> io::Result<Socket> {
        let socket = self.new_socket().await;

        socket.tx(PortCommand::BindUdp {
            handle: socket.handle,
            local_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;

        Ok(socket)
    }

    pub async fn bind_icmp(&self, local_addr: SocketAddr) -> io::Result<Socket> {
        let socket = self.new_socket().await;

        socket.tx(PortCommand::BindIcmp {
            handle: socket.handle,
            local_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;

        Ok(socket)
    }

    pub async fn bind_dhcp(&self, lease_duration: Option<Duration>, ignore_naks: bool) -> io::Result<Socket> {
        let socket = self.new_socket().await;

        socket.tx(PortCommand::BindDhcp {
            handle: socket.handle,
            lease_duration,
            ignore_naks,
        }).await?;

        Ok(socket)
    }

    pub async fn connect_tcp(&self, local_addr: SocketAddr, peer_addr: SocketAddr) -> io::Result<Socket> {
        let socket = self.new_socket().await;

        socket.tx(PortCommand::ConnectTcp {
            handle: socket.handle,
            local_addr,
            peer_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;

        Ok(socket)
    }

    pub async fn accept_tcp(&self, listen_addr: SocketAddr) -> io::Result<(Socket, SocketAddr)> {
        let mut socket = self.new_socket().await;

        socket.tx(PortCommand::Listen {
            handle: socket.handle,
            local_addr: listen_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;

        let peer_addr = socket.accept().await?;
        Ok((socket, peer_addr))
    }

    pub async fn run(&self) {
        let mut rx = self.rx.lock().await;
        while let Some(evt) = rx.recv().await {
            if let Ok(evt) = bincode::deserialize::<PortResponse>(&evt[..]) {
                let sockets = self.sockets.lock().await;
                match evt {
                    PortResponse::Received {
                        handle,
                        data
                    } => {
                        if let Some(socket) = sockets.get(&handle.0) {
                            let _ = socket.recv.send(EventRecv { data }).await;
                        }
                    }
                    PortResponse::ReceivedFrom {
                        handle,
                        peer_addr,
                        data,
                    } => {
                        if let Some(socket) = sockets.get(&handle.0) {
                            let _ = socket.recv_from.send(EventRecvFrom { peer_addr, data }).await;
                        }
                    }
                    PortResponse::TcpAccepted {
                        handle,
                        peer_addr,
                    } => {
                        if let Some(socket) = sockets.get(&handle.0) {
                            let _ = socket.accept.send(EventAccept { peer_addr }).await;
                        }
                    }
                    PortResponse::SocketError {
                        handle,
                        error,
                    } => {
                        if let Some(socket) = sockets.get(&handle.0) {
                            let _ = socket.error.send(EventError { error }).await;
                        }
                    }
                    PortResponse::DhcpDeconfigured {
                        handle,
                    } => {
                        if let Some(socket) = sockets.get(&handle.0) {
                            let _ = socket.deconfigure.send(EventDhcpDeconfigured { }).await;
                        }
                    }
                    PortResponse::DhcpConfigured {
                        handle,
                        address,
                        router,
                        dns_servers,
                    } => {
                        if let Some(socket) = sockets.get(&handle.0) {
                            let _ = socket.configure.send(EventDhcpConfigured { address, router, dns_servers }).await;
                        }
                    }
                }
            }
        }
    }
}