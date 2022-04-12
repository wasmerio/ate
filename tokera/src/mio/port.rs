use ate::chain::ChainKey;
use ate::crypto::EncryptKey;
use std::io;
use std::time::Duration;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::BTreeMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use ate::comms::StreamTx;
use ate::comms::StreamRx;

use crate::api::InstanceClient;
use crate::model::IpVersion;
use crate::model::IpProtocol;
use crate::model::PortCommand;
use crate::model::PortResponse;
use crate::model::SocketHandle;
use crate::model::SwitchHello;

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
    tx: Arc<Mutex<StreamTx>>,
    rx: Arc<Mutex<StreamRx>>,
    ek: Option<EncryptKey>,
    sockets: Arc<Mutex<BTreeMap<i32, SocketState>>>,
}

impl Port
{
    pub async fn new(url: url::Url, chain: ChainKey, access_token: String,) -> io::Result<Port>
    {
        let client = InstanceClient::new(url).await
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        let (mut tx, rx, ek) = client.split();

        let hello = SwitchHello {
            chain,
            access_token,
            version: crate::model::PORT_COMMAND_VERSION,
        };

        let data = serde_json::to_vec(&hello)?;
        tx.send(&ek, &data[..]).await?;

        Ok(Port {
            tx: Arc::new(Mutex::new(tx)),
            rx: Arc::new(Mutex::new(rx)),
            ek,
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
            ek: self.ek.clone(),
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
        let mut total_read = 0u64;
        let mut rx = self.rx.lock().await;
        while let Ok(evt) = rx.read_buf_with_header(&self.ek, &mut total_read).await {
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