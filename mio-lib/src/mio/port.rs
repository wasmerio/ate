use ate_crypto::EncryptKey;
use chrono::DateTime;
use chrono::Utc;
use std::io;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::BTreeMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::model::IpCidr;
use crate::model::IpRoute;
use crate::model::IpProtocol;
use crate::model::PortCommand;
use crate::model::PortResponse;
use crate::model::PortNopType;
use crate::model::SocketHandle;

pub type StreamRx = Box<dyn crate::comms::StreamReceiver + Send + Sync>;
pub type StreamTx = Box<dyn crate::comms::StreamTransmitter + Send + Sync>;

use super::evt::*;
use super::socket::*;

const MAX_MPSC: usize = std::usize::MAX >> 3;

pub struct SocketState
{
    nop: mpsc::Sender<PortNopType>,
    recv: mpsc::Sender<EventRecv>,
    recv_from: mpsc::Sender<EventRecvFrom>,
    error: mpsc::Sender<EventError>,
    accept: mpsc::Sender<EventAccept>,
}

#[derive(Default)]
pub struct PortState
{
    addresses: Vec<IpCidr>,
    routes: Vec<IpRoute>,
    router: Option<IpAddr>,
    dns_servers: Vec<IpAddr>,
    sockets: BTreeMap<i32, SocketState>,
    dhcp_client: Option<Socket>,
}

pub struct Port
{
    tx: Arc<Mutex<StreamTx>>,
    ek: Option<EncryptKey>,
    state: Arc<Mutex<PortState>>,
}

impl Port
{
    pub async fn new(tx: StreamTx, rx: StreamRx, ek: Option<EncryptKey>) -> io::Result<Port>
    {
        let (init_tx, mut init_rx) = mpsc::channel(1);
        let state = Arc::new(Mutex::new(PortState::default()));

        {
            let ek = ek.clone();
            let state = state.clone();
            tokio::task::spawn(async move {
                Self::run(rx, ek, state, init_tx).await
            });
        }

        let port = Port {
            tx: Arc::new(Mutex::new(tx)),
            ek,
            state,
        };

        port.tx(PortCommand::Init).await?;
        init_rx.recv().await
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "failed to initialize the socket before it was closed."))?;

        Ok(port)
    }

    async fn new_socket(&self, proto: Option<IpProtocol>) -> Socket {
        let mut state = self.state.lock().await;
        let sockets = &mut state.sockets;
        let handle = sockets.iter()
            .rev()
            .next()
            .map(|e| e.0.clone() + 1)
            .unwrap_or_else(|| 1i32);
        
        let (tx_nop, rx_nop) = mpsc::channel(MAX_MPSC);
        let (tx_recv, rx_recv) = mpsc::channel(MAX_MPSC);
        let (tx_recv_from, rx_recv_from) = mpsc::channel(MAX_MPSC);
        let (tx_error, rx_error) = mpsc::channel(MAX_MPSC);
        let (tx_accept, rx_accept) = mpsc::channel(MAX_MPSC);
        
        sockets.insert(handle, SocketState{
            nop: tx_nop,
            recv: tx_recv,
            recv_from: tx_recv_from,
            error: tx_error,
            accept: tx_accept,
        });

        let handle = SocketHandle(handle);
        Socket {
            handle,
            proto,
            peer_addr: None,
            tx: self.tx.clone(),
            ek: self.ek.clone(),
            nop: rx_nop,
            recv: rx_recv,
            recv_from: rx_recv_from,
            error: rx_error,
            accept: rx_accept,
        }
    }

    pub async fn bind_raw(&self) -> io::Result<Socket> {
        let mut socket = self.new_socket(None).await;

        socket.tx(PortCommand::BindRaw {
            handle: socket.handle,
        }).await?;
        socket.nop(PortNopType::BindRaw).await?;

        Ok(socket)
    }

    pub async fn bind_udp(&self, local_addr: SocketAddr) -> io::Result<Socket> {
        let mut socket = self.new_socket(Some(IpProtocol::Udp)).await;

        socket.tx(PortCommand::BindUdp {
            handle: socket.handle,
            local_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;
        socket.nop(PortNopType::BindUdp).await?;

        Ok(socket)
    }

    pub async fn bind_icmp(&self, local_addr: SocketAddr) -> io::Result<Socket> {
        let mut socket = self.new_socket(Some(IpProtocol::Icmp)).await;

        socket.tx(PortCommand::BindIcmp {
            handle: socket.handle,
            local_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;
        socket.nop(PortNopType::BindIcmp).await?;

        Ok(socket)
    }

    pub async fn bind_dhcp(&self) -> io::Result<Socket> {
        let mut socket = self.new_socket(Some(IpProtocol::Icmp)).await;

        socket.tx(PortCommand::BindDhcp {
            handle: socket.handle,
            lease_duration: None,
            ignore_naks: false,
        }).await?;
        socket.nop(PortNopType::BindDhcp).await?;

        Ok(socket)
    }

    pub async fn connect_tcp(&self, local_addr: SocketAddr, peer_addr: SocketAddr) -> io::Result<Socket> {
        let mut socket = self.new_socket(Some(IpProtocol::Tcp)).await;

        socket.tx(PortCommand::ConnectTcp {
            handle: socket.handle,
            local_addr,
            peer_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;
        socket.nop(PortNopType::ConnectTcp).await?;

        socket.wait_till_may_send().await?;

        Ok(socket)
    }

    pub async fn listen_tcp(&self, listen_addr: SocketAddr) -> io::Result<Socket> {
        let mut socket = self.new_socket(Some(IpProtocol::Tcp)).await;

        socket.tx(PortCommand::Listen {
            handle: socket.handle,
            local_addr: listen_addr,
            hop_limit: Socket::HOP_LIMIT,
        }).await?;
        socket.nop(PortNopType::Listen).await?;

        Ok(socket)
    }

    pub async fn dhcp_acquire(&self) -> io::Result<Ipv4Addr> {
        let mut socket = self.bind_dhcp().await?;
        socket.nop(PortNopType::DhcpAcquire).await?;

        let mut state = self.state.lock().await;
        state.dhcp_client = Some(socket);
        state.addresses
            .clone()
            .into_iter()
            .filter_map(|cidr| match &cidr.ip {
                IpAddr::V4(a) => Some(a.clone()),
                _ => None,
            })
            .next()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::AddrNotAvailable, "dhcp server did not return an IP address")
            })
    }

    pub async fn add_ip(&mut self, ip: IpAddr, prefix: u8) -> io::Result<IpCidr> {
        let cidr = IpCidr {
            ip,
            prefix,
        };
        {
            let mut state = self.state.lock().await;
            state.addresses.push(cidr.clone());
        }
        self.update_ips().await?;
        Ok(cidr)
    }

    pub async fn remove_ip(&mut self, ip: IpAddr) -> io::Result<Option<IpCidr>> {
        let ret = {
            let mut state = self.state.lock().await;
            let ret = state.addresses.iter().filter(|cidr| cidr.ip == ip).map(|cidr| cidr.clone()).next();
            state.addresses.retain(|cidr| cidr.ip != ip);
            state.routes.retain(|route| route.cidr.ip != ip);
            ret
        };
        self.update_ips().await?;
        self.update_routes().await?;
        Ok(ret)
    }

    pub async fn addresses(&self) -> Vec<IpCidr> {
        let state = self.state.lock().await;
        state.addresses.clone()
    }

    pub async fn clear_addresses(&mut self) -> io::Result<()> {
        {
            let mut state = self.state.lock().await;
            state.addresses.clear();
            state.routes.clear();
        }
        self.update_ips().await?;
        self.update_routes().await?;
        Ok(())
    }

    async fn update_ips(&mut self) -> io::Result<()> {
        let addrs = {
            let state = self.state.lock().await;
            state.addresses.clone()
        };
        self.tx(PortCommand::SetAddresses { addrs }).await
    }

    pub async fn add_default_route(&mut self, gateway: IpAddr) -> io::Result<IpRoute> {
        let cidr = IpCidr {
            ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            prefix: 0
        };
        self.add_route(cidr, gateway, None, None).await
    }


    pub async fn add_route(&mut self, cidr: IpCidr, via_router: IpAddr, preferred_until: Option<DateTime<Utc>>, expires_at: Option<DateTime<Utc>>) -> io::Result<IpRoute> {
        let route = IpRoute {
            cidr,
            via_router,
            preferred_until,
            expires_at
        };
        {
            let mut state = self.state.lock().await;
            state.routes.push(route.clone());
        }
        self.update_routes().await?;
        Ok(route)
    }

    pub async fn remove_route_by_address(&mut self, addr: IpAddr) -> io::Result<Option<IpRoute>> {
        let ret = {
            let mut state = self.state.lock().await;
            let ret = state.routes.iter().filter(|route| route.cidr.ip == addr).map(|route| route.clone()).next();
            state.routes.retain(|route| route.cidr.ip != addr);
            ret
        };
        self.update_routes().await?;
        Ok(ret)
    }

    pub async fn remove_route_by_gateway(&mut self, gw_ip: IpAddr) -> io::Result<Option<IpRoute>> {
        let ret = {
            let mut state = self.state.lock().await;
            let ret = state.routes.iter().filter(|route| route.via_router == gw_ip).map(|route| route.clone()).next();
            state.routes.retain(|route| route.via_router != gw_ip);
            ret
        };
        self.update_routes().await?;
        Ok(ret)
    }

    pub async fn route_table(&self) -> Vec<IpRoute> {
        let state = self.state.lock().await;
        state.routes.clone()
    }

    pub async fn clear_route_table(&mut self) -> io::Result<()> {
        {
            let mut state = self.state.lock().await;
            state.routes.clear();
        }
        self.update_routes().await?;
        Ok(())
    }

    async fn update_routes(&mut self) -> io::Result<()> {
        let routes = {
            let state = self.state.lock().await;
            state.routes.clone()
        };
        self.tx(PortCommand::SetRoutes { routes }).await
    }

    pub async fn addr_ipv4(&self) -> Option<Ipv4Addr>
    {
        let state = self.state.lock().await;
        state.addresses
            .iter()
            .filter_map(|cidr| {
                match cidr.ip {
                    IpAddr::V4(a) => Some(a.clone()),
                    _ => None
                }
            })
            .next()
    }

    pub async fn addr_ipv6(&self) -> Vec<Ipv6Addr>
    {
        let state = self.state.lock().await;
        state.addresses
            .iter()
            .filter_map(|cidr| {
                match &cidr.ip {
                    IpAddr::V6(a) => Some(a.clone()),
                    _ => None
                }
            })
            .collect()
    }

    async fn tx(&self, cmd: PortCommand) -> io::Result<()> {
        let cmd = bincode::serialize(&cmd)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let mut tx = self.tx.lock().await;
        tx.send(&self.ek, &cmd[..]).await?;
        Ok(())
    }

    async fn run(mut rx: StreamRx, ek: Option<EncryptKey>, state: Arc<Mutex<PortState>>, init_tx: mpsc::Sender<()>) {
        while let Ok(evt) = rx.recv(&ek).await {
            if let Ok(evt) = bincode::deserialize::<PortResponse>(&evt[..]) {
                let mut state = state.lock().await;
                match evt {
                    PortResponse::Nop {
                        handle,
                        ty
                    } => {
                        if let Some(socket) = state.sockets.get(&handle.0) {
                            let _ = socket.nop.send(ty).await;
                        }
                    }
                    PortResponse::Received {
                        handle,
                        data
                    } => {
                        if let Some(socket) = state.sockets.get(&handle.0) {
                            let _ = socket.recv.send(EventRecv { data }).await;
                        }
                    }
                    PortResponse::ReceivedFrom {
                        handle,
                        peer_addr,
                        data,
                    } => {
                        if let Some(socket) = state.sockets.get(&handle.0) {
                            let _ = socket.recv_from.send(EventRecvFrom { peer_addr, data }).await;
                        }
                    }
                    PortResponse::TcpAccepted {
                        handle,
                        peer_addr,
                    } => {
                        if let Some(socket) = state.sockets.get(&handle.0) {
                            let _ = socket.accept.send(EventAccept { peer_addr }).await;
                        }
                    }
                    PortResponse::SocketError {
                        handle,
                        error,
                    } => {
                        if let Some(socket) = state.sockets.get(&handle.0) {
                            let _ = socket.error.send(EventError { error }).await;
                        }
                    }
                    PortResponse::CidrTable {
                        cidrs
                    } => {
                        state.addresses = cidrs;
                    }
                    PortResponse::RouteTable {
                        routes
                    } => {
                        state.routes = routes;
                    }
                    PortResponse::DhcpDeconfigured {
                        handle: _,
                    } => {
                        state.addresses.clear();
                        state.router = None;
                        state.dns_servers.clear();
                    }
                    PortResponse::DhcpConfigured {
                        handle: _,
                        address,
                        router,
                        dns_servers,
                    } => {
                        state.addresses.retain(|cidr| cidr.ip != address.ip);
                        state.addresses.push(address);
                        state.router = router;
                        state.dns_servers = dns_servers;
                    }
                    PortResponse::Inited => {
                        let _ = init_tx.send(()).await;
                    }
                }
            }
        }
        debug!("mio port closed");
        
        // Clearing the sockets will shut them all down
        let mut state = state.lock().await;
        state.dhcp_client = None;
        state.sockets.clear();
    }
}