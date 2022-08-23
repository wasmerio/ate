use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use smoltcp::iface::NeighborCache;
use smoltcp::iface::Routes;
use smoltcp::time::Instant;
use smoltcp::socket::Dhcpv4Event;
use smoltcp::wire::Ipv4Cidr;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::broadcast;
use crossbeam::queue::SegQueue;
use derivative::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use wasmer_deploy_cli::model::PortCommand;
use wasmer_deploy_cli::model::PortResponse;
use wasmer_deploy_cli::model::PortNopType;
use wasmer_deploy_cli::model::HardwareAddress;
use wasmer_deploy_cli::model::SocketHandle;
use wasmer_deploy_cli::model::SocketError;
use wasmer_deploy_cli::model::SocketErrorKind;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::IpCidr;
use smoltcp::iface;
use smoltcp::iface::Interface;
use smoltcp::iface::InterfaceBuilder;
use smoltcp::iface::Route;
use smoltcp::phy;
use smoltcp::phy::Device;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::phy::Medium;
use smoltcp::phy::ChecksumCapabilities;
use smoltcp::socket::IcmpEndpoint;
use smoltcp::socket::IcmpSocket;
use smoltcp::socket::IcmpSocketBuffer;
use smoltcp::socket::IcmpPacketMetadata;
use smoltcp::socket::Dhcpv4Socket;
use smoltcp::socket::TcpSocket;
use smoltcp::socket::TcpSocketBuffer;
use smoltcp::socket::UdpSocket;
use smoltcp::socket::UdpSocketBuffer;
use smoltcp::socket::UdpPacketMetadata;
use managed::ManagedSlice;
use managed::ManagedMap;

use super::switch::*;
use super::raw::*;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Port
{
    pub(crate) switch: Arc<Switch>,
    pub(crate) wake: watch::Receiver<()>,
    pub(crate) raw_tx: broadcast::Sender<Vec<u8>>,
    pub(crate) raw_mode: bool,
    #[allow(dead_code)]
    #[derivative(Debug = "ignore")]
    pub(crate) mac: EthernetAddress,
    pub(crate) mac_drop: mpsc::Sender<HardwareAddress>,
    pub(crate) listen_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) tcp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) udp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) raw_sockets: HashMap<SocketHandle, TapSocket>,
    pub(crate) icmp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) dhcp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    #[derivative(Debug = "ignore")]
    pub(crate) iface: Interface<'static, PortDevice>,
    pub(crate) buf_size: usize,
    pub(crate) tx_queue: Vec<PortResponse>,
}

impl Port
{
    pub fn new(switch: &Arc<Switch>, mac: HardwareAddress, data: Arc<SegQueue<Vec<u8>>>, wake: watch::Receiver<()>, mac_drop: mpsc::Sender<HardwareAddress>, raw_tx: broadcast::Sender<Vec<u8>>) -> Port {
        let mac = EthernetAddress::from_bytes(mac.as_bytes());
        let device = PortDevice {
            data,
            mac,
            mtu: 1500,
            switch: Arc::clone(switch)
        };

        // Create the neighbor cache and add the broadcast address which will expire in 100 years (i.e. never).
        let neighbor_cache = NeighborCache::new(BTreeMap::new());
        let iface = InterfaceBuilder::new(device, vec![])
            .hardware_addr(mac.into())
            .neighbor_cache(neighbor_cache)
            .ip_addrs(Vec::new())
            .routes(Routes::new(BTreeMap::default()))
            .random_seed(fastrand::u64(..));
        let iface = iface.finalize();
        
        Port {
            switch: Arc::clone(switch),
            raw_tx, 
            wake,
            raw_mode: false,
            mac,
            mac_drop,
            udp_sockets: Default::default(),
            tcp_sockets: Default::default(),
            listen_sockets: Default::default(),
            raw_sockets: Default::default(),
            icmp_sockets: Default::default(),
            dhcp_sockets: Default::default(),
            buf_size: 16,
            iface,
            tx_queue: Vec::new(),
        }
    }

    fn queue_error(&mut self, handle: SocketHandle, error: SocketErrorKind) {
        self.queue_tx(
            PortResponse::SocketError
            {
                handle,
                error: SocketError::Simple(error)
            }
        );
    }

    fn queue_nop(&mut self, handle: SocketHandle, ty: PortNopType) {
        self.queue_tx(PortResponse::Nop { handle, ty });
    }

    fn queue_tx(&mut self, msg: PortResponse) {
        self.tx_queue.push(msg);
    }

    pub fn process(&mut self, action: PortCommand) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            PortCommand::Send {
                handle,
                data
            } => {
                if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..]) {
                        self.queue_error(handle, conv_err(err));
                    }
                } else if let Some(raw_socket) = self.raw_sockets.get(&handle) {
                    raw_socket.send(data);
                } else if self.udp_sockets.contains_key(&handle) {
                    self.queue_error(handle, SocketErrorKind::Unsupported);
                } else if self.icmp_sockets.contains_key(&handle) {
                    self.queue_error(handle, SocketErrorKind::Unsupported);
                } else {
                    self.queue_error(handle, SocketErrorKind::NotConnected);
                }
            },
            PortCommand::SendTo {
                handle,
                data,
                addr,
            } => {
                if unicast_good(addr.ip()) == false {
                    self.queue_error(handle, SocketErrorKind::HostUnreachable);
                    return Ok(());
                }
                if let Some(socket_handle) = self.udp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<UdpSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..], addr.into()) {
                        self.queue_error(handle, conv_err(err));
                    }
                } else if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..]) {
                        self.queue_error(handle, conv_err(err));
                    }
                } else if let Some(socket_handle) = self.icmp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<IcmpSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..], addr.ip().into()) {
                        self.queue_error(handle, conv_err(err));
                    }
                } else if self.raw_sockets.contains_key(&handle) {
                    self.queue_error(handle, SocketErrorKind::Unsupported);
                } else {
                    self.queue_error(handle, SocketErrorKind::NotConnected);
                }
            },
            PortCommand::MaySend {
                handle,
            } => {
                let mut err = SocketErrorKind::WouldBlock;
                let may_send = {
                    if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                        let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                        socket.may_send()
                    } else if let Some(_) = self.udp_sockets.get(&handle) {
                        true
                    } else if let Some(_) = self.icmp_sockets.get(&handle) {
                        true
                    } else if let Some(_) = self.raw_sockets.get(&handle) {
                        true
                    } else {
                        err = SocketErrorKind::NotConnected;
                        false
                    }
                };
                if may_send {
                    self.queue_nop(handle, PortNopType::MaySend);
                } else {
                    self.queue_error(handle, err);
                }
            },
            PortCommand::MayReceive {
                handle,
            } => {
                let mut err = SocketErrorKind::WouldBlock;
                let may_receive = {
                    if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                        let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                        socket.may_recv()
                    } else if let Some(_) = self.udp_sockets.get(&handle) {
                        true
                    } else if let Some(_) = self.icmp_sockets.get(&handle) {
                        true
                    } else if let Some(_) = self.raw_sockets.get(&handle) {
                        true
                    } else {
                        err = SocketErrorKind::NotConnected;
                        false
                    }
                };
                if may_receive {
                    self.queue_nop(handle, PortNopType::MayReceive);
                } else {
                    self.queue_error(handle, err);
                }
            },
            PortCommand::CloseHandle {
                handle,
            } => {
                if let Some(socket_handle) = self.udp_sockets.remove(&handle) {
                    let socket = self.iface.get_socket::<UdpSocket>(socket_handle);
                    socket.close();
                    drop(socket);
                    self.iface.remove_socket(socket_handle);
                }
                if let Some(socket_handle) = self.tcp_sockets.remove(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(socket_handle);
                    socket.close();
                    drop(socket);
                    self.iface.remove_socket(socket_handle);
                }
                if let Some(socket_handle) = self.listen_sockets.remove(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(socket_handle);
                    socket.close();
                    drop(socket);
                    self.iface.remove_socket(socket_handle);
                }
                if let Some(socket_handle) = self.icmp_sockets.remove(&handle) {
                    self.iface.remove_socket(socket_handle);
                }
                if let Some(_) = self.raw_sockets.remove(&handle) {
                }
                if let Some(socket_handle) = self.dhcp_sockets.remove(&handle) {
                    self.iface.remove_socket(socket_handle);
                } 
                self.queue_nop(handle, PortNopType::CloseHandle);
            },
            PortCommand::BindRaw {
                handle,
            } => {
                match self.switch_to_raw() {
                    Ok(()) => {
                        let mac = HardwareAddress::from_bytes(self.mac.as_bytes());
                        let rx = self.raw_tx.subscribe();
                        let raw_socket = TapSocket::new(&self.switch, mac, rx);
                        self.raw_sockets.insert(handle, raw_socket);
                        self.queue_nop(handle, PortNopType::BindRaw);
                    },
                    Err(err) => {
                        debug!("{}", err);
                        self.queue_error(handle, SocketErrorKind::InvalidInput);
                    }
                };
            },
            PortCommand::BindIcmp {
                handle,
                local_addr,
                hop_limit,
            } => {
                match self.switch_to_smoltcp() {
                    Ok(()) => {
                        let rx_buffer = IcmpSocketBuffer::new(icmp_meta_buf(self.buf_size), self.raw_buf(1));
                        let tx_buffer = IcmpSocketBuffer::new(icmp_meta_buf(self.buf_size), self.raw_buf(1));
                        let mut socket = IcmpSocket::new(rx_buffer, tx_buffer);
                        socket.set_hop_limit(Some(hop_limit));
                        if let Err(err) = socket.bind(IcmpEndpoint::Ip(local_addr.into())) {
                            self.queue_error(handle, conv_err(err));
                        } else {
                            self.icmp_sockets.insert(handle, self.iface.add_socket(socket));
                            self.queue_nop(handle, PortNopType::BindIcmp);
                        }
                    },
                    Err(err) => {
                        debug!("{}", err);
                        self.queue_error(handle, SocketErrorKind::InvalidInput);
                    }
                }
            },
            PortCommand::BindDhcp {
                handle,
                lease_duration,
                ignore_naks,
            } => {
                let lease_duration = lease_duration.map(|d| smoltcp::time::Duration::from_micros(d.as_micros() as u64));

                let mut socket = Dhcpv4Socket::new();
                socket.set_max_lease_duration(lease_duration);
                socket.set_ignore_naks(ignore_naks);
                socket.reset();
                self.dhcp_sockets.insert(handle, self.iface.add_socket(socket));
                self.queue_nop(handle, PortNopType::BindDhcp);
            },
            PortCommand::DhcpReset {
                handle,
            } => {
                if let Some(socket_handle) = self.dhcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<Dhcpv4Socket>(*socket_handle);
                    socket.reset();
                }
                self.queue_nop(handle, PortNopType::DhcpReset);
            },
            PortCommand::BindUdp {
                handle,
                local_addr,
                hop_limit,
            } => {
                match self.switch_to_smoltcp() {
                    Ok(()) => {
                        if unicast_good(local_addr.ip()) == false {
                            self.queue_error(handle, SocketErrorKind::AddrNotAvailable);
                            return Ok(());
                        }
                        let rx_buffer = UdpSocketBuffer::new(udp_meta_buf(self.buf_size), self.ip_buf(1));
                        let tx_buffer = UdpSocketBuffer::new(udp_meta_buf(self.buf_size), self.ip_buf(1));
                        let mut socket = UdpSocket::new(rx_buffer, tx_buffer);
                        socket.set_hop_limit(Some(hop_limit));
                        if let Err(err) = socket.bind(local_addr) {
                            self.queue_error(handle, conv_err(err));
                        } else {
                            self.udp_sockets.insert(handle, self.iface.add_socket(socket));
                            self.queue_nop(handle, PortNopType::BindUdp);
                        }
                    },
                    Err(err) => {
                        debug!("{}", err);
                        self.queue_error(handle, SocketErrorKind::InvalidInput);
                    }
                }
            },
            PortCommand::ConnectTcp {
                handle,
                local_addr,
                peer_addr,
                hop_limit,
            } => {
                match self.switch_to_smoltcp() {
                    Ok(()) => {
                        if unicast_good(local_addr.ip()) == false {
                            self.queue_error(handle, SocketErrorKind::AddrNotAvailable);
                            return Ok(());
                        }
                        if unicast_good(peer_addr.ip()) == false {
                            self.queue_error(handle, SocketErrorKind::HostUnreachable);
                            return Ok(());
                        }
                        let rx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                        let tx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                        let mut socket = TcpSocket::new(rx_buffer, tx_buffer);
                        socket.set_hop_limit(Some(hop_limit));
                        let socket_handle = self.iface.add_socket(socket);
                        let (socket, cx) = self.iface.get_socket_and_context::<TcpSocket>(socket_handle);
                        if let Err(err) = socket.connect(cx, peer_addr, local_addr) {
                            self.queue_error(handle, conv_err(err));
                        } else {
                            self.tcp_sockets.insert(handle, socket_handle);
                            self.queue_nop(handle, PortNopType::ConnectTcp);
                        }
                    },
                    Err(err) => {
                        debug!("{}", err);
                        self.queue_error(handle, SocketErrorKind::InvalidInput);
                    }
                }
            },
            PortCommand::Listen {
                handle,
                local_addr,
                hop_limit,
            } => {
                match self.switch_to_smoltcp() {
                    Ok(()) => {
                        if unicast_good(local_addr.ip()) == false {
                            self.queue_error(handle, SocketErrorKind::AddrNotAvailable);
                            return Ok(());
                        }
                        let rx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                        let tx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                        let mut socket = TcpSocket::new(rx_buffer, tx_buffer);
                        socket.set_hop_limit(Some(hop_limit));
                        if let Err(err) = socket.listen(local_addr) {
                            self.queue_error(handle, conv_err(err));
                        } else {
                            self.listen_sockets.insert(handle, self.iface.add_socket(socket));
                            self.queue_nop(handle, PortNopType::Listen);
                        }
                    },
                    Err(err) => {
                        debug!("{}", err);
                        self.queue_error(handle, SocketErrorKind::InvalidInput);
                    }
                }
            },
            PortCommand::SetHopLimit {
                handle,
                hop_limit
            } => {
                if let Some(socket_handle) = self.udp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<UdpSocket>(*socket_handle);
                    socket.set_hop_limit(Some(hop_limit));
                }
                if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_hop_limit(Some(hop_limit));
                }
                if let Some(socket_handle) = self.listen_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_hop_limit(Some(hop_limit));
                }
                self.queue_nop(handle, PortNopType::SetHopLimit);
            },
            PortCommand::SetAckDelay {
                handle,
                duration_ms,
            } => {
                let duration = smoltcp::time::Duration::from_millis(duration_ms as u64);
                if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_ack_delay(Some(duration.clone()));
                }
                if let Some(socket_handle) = self.listen_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_ack_delay(Some(duration.clone()));
                }
                self.queue_nop(handle, PortNopType::SetAckDelay);
            },
            PortCommand::SetNoDelay {
                handle,
                no_delay,
            } => {
                let nagle_enable = no_delay == false;
                if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_nagle_enabled(nagle_enable);
                }
                if let Some(socket_handle) = self.listen_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_nagle_enabled(nagle_enable);
                }
                self.queue_nop(handle, PortNopType::SetNoDelay);
            },
            PortCommand::SetPromiscuous {
                handle,
                promiscuous,
            } => {
                if let Some(socket) = self.raw_sockets.get_mut(&handle) {
                    socket.set_promiscuous(promiscuous);
                }
                self.queue_nop(handle, PortNopType::SetPromiscuous);
            },
            PortCommand::SetKeepAlive {
                handle,
                interval,
            } => {
                let interval = interval.map(|d| smoltcp::time::Duration::from_micros(d.as_micros() as u64));
                if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_keep_alive(interval.into())
                }
                if let Some(socket_handle) = self.listen_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_keep_alive(interval.into())
                }
                self.queue_nop(handle, PortNopType::SetKeepAlive);
            },
            PortCommand::SetTimeout {
                handle,
                timeout,
            } => {
                let timeout = timeout.map(|d| smoltcp::time::Duration::from_micros(d.as_micros() as u64));
                if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_timeout(timeout.into())
                }
                if let Some(socket_handle) = self.listen_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    socket.set_timeout(timeout.into())
                }
                self.queue_nop(handle, PortNopType::SetTimeout);
            },
            PortCommand::JoinMulticast {
                multiaddr,
            } => {
                let timestamp = Instant::now();
                self.iface.join_multicast_group(multiaddr, timestamp)?;
            },
            PortCommand::LeaveMulticast {
                multiaddr,
            } => {
                let timestamp = Instant::now();
                self.iface.leave_multicast_group(multiaddr, timestamp)?;
            },
            PortCommand::SetHardwareAddress {
                mac,
            } => {
                self.iface.set_hardware_addr(EthernetAddress::from_bytes(mac.as_bytes()).into());
            },
            PortCommand::SetAddresses {
                addrs: ips,
            } => {
                self.iface.update_ip_addrs(|target| {
                    // Anything that is not a unicast will cause a panic
                    let mut ips = ips.into_iter()
                        .filter(|route| cidr_good(route.ip))
                        .map(|cidr| {
                            IpCidr::new(cidr.ip.into(), cidr.prefix)
                        })
                        .collect();

                    // Replace the IP addresses
                    if let ManagedSlice::Owned(vec) = target {
                        vec.clear();
                        vec.append(&mut ips);
                    }
                });
            },
            PortCommand::SetRoutes {
                routes,
            } => {
                let iface_routes = self.iface.routes_mut();
                iface_routes.update(|target| {
                    let mut routes: BTreeMap<IpCidr, Route> = routes.into_iter()
                        .map(|route| {
                            (
                                IpCidr::new(route.cidr.ip.into(), route.cidr.prefix),
                                Route {
                                    via_router: route.via_router.into(),
                                    preferred_until: route.preferred_until.map(|d| {
                                        let diff = d.signed_duration_since(wasmer_deploy_cli::model::Utc::now());
                                        Instant::from_micros(Instant::now().micros() + diff.num_microseconds().unwrap_or(0))
                                    }),
                                    expires_at: route.expires_at.map(|d| {
                                        let diff = d.signed_duration_since(wasmer_deploy_cli::model::Utc::now());
                                        Instant::from_micros(Instant::now().micros() + diff.num_microseconds().unwrap_or(0))
                                    })
                                }
                            )
                        })
                        .collect();

                    // Replace the routes
                    if let ManagedMap::Owned(map) = target {
                        map.clear();
                        map.append(&mut routes);
                    }
                });
            },
            PortCommand::Init => {
                self.queue_tx(PortResponse::Inited {
                    mac: HardwareAddress::from_bytes(self.mac.as_bytes())
                });
            },
        }
        Ok(())
    }

    pub fn switch_to_smoltcp(&mut self) -> Result<(), &str> {
        let mut state = self.switch.data_plane.lock().unwrap();
        if let Some(dst) = state.ports.remove(&self.mac) {
            match dst {
                Destination::LocalSmoltcp(port) => {
                    state.ports.insert(self.mac.clone(), Destination::LocalSmoltcp(port));
                    Ok(())
                }
                Destination::LocalRaw(port) => {
                    state.ports.insert(self.mac.clone(), Destination::LocalRaw(port));
                    Err("port is already in a raw mode")
                }
                Destination::LocalDuel(port_smoltcp, _) => {
                    state.ports.insert(self.mac.clone(), Destination::LocalSmoltcp(port_smoltcp));
                    Ok(())
                }
                _ => {
                    Err("port is in an invalid state to switch")
                }
            }
        } else {
            Err("failed to switch port mode as it is unknown")
        }
    }

    pub fn switch_to_raw(&mut self) -> Result<(), &str> {
        let mut state = self.switch.data_plane.lock().unwrap();
        if let Some(dst) = state.ports.remove(&self.mac) {
            match dst {
                Destination::LocalSmoltcp(port) => {
                    state.ports.insert(self.mac.clone(), Destination::LocalSmoltcp(port));
                    Err("port is already in a smoltcp mode")
                }
                Destination::LocalRaw(port) => {
                    state.ports.insert(self.mac.clone(), Destination::LocalRaw(port));
                    Ok(())
                }
                Destination::LocalDuel(_, port_raw) => {
                    state.ports.insert(self.mac.clone(), Destination::LocalRaw(port_raw));
                    Ok(())
                }
                _ => {
                    Err("port is in an invalid state to switch")
                }
            }
        } else {
            Err("failed to switch port mode as it is unknown")
        }
    }

    pub fn poll(&mut self) -> (Vec<PortResponse>, Duration) {
        let readiness = self.iface
            .poll(Instant::now())
            .unwrap_or(false);

        let wait_time = self.iface
            .poll_delay(Instant::now())
            .unwrap_or(smoltcp::time::Duration::ZERO);

        let mut ret = Vec::new();
        
        if readiness {
            for (handle, socket_handle) in self.udp_sockets.iter() {
                let socket = self.iface.get_socket::<UdpSocket>(*socket_handle);
                while socket.can_recv() {
                    if let Ok((data, addr)) = socket.recv() {
                        let data = data.to_vec();
                        let peer_addr = conv_addr(addr);
                        ret.push(PortResponse::ReceivedFrom { handle: *handle, peer_addr, data });
                    } else {
                        break;
                    }
                }
            }

            for (handle, socket_handle) in self.tcp_sockets.iter() {
                let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                if socket.can_recv() {
                    let mut data = Vec::new();
                    while socket.can_recv() {
                        if socket.recv(|d| {
                            data.extend_from_slice(d);
                            (d.len(), d.len())
                        }).is_err() {
                            break;
                        }
                    }
                    if data.len() > 0 {
                        ret.push(PortResponse::Received { handle: *handle, data });
                    }
                }
            }

            let mut move_me = Vec::new();
            for (handle, socket_handle) in self.listen_sockets.iter() {
                let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                if socket.is_listening() == false {
                    if socket.is_active() {
                        let peer_addr = conv_addr(socket.remote_endpoint());
                        ret.push(PortResponse::TcpAccepted { handle: *handle, peer_addr });
                    } else {
                        ret.push(PortResponse::SocketError { handle: *handle, error: SocketErrorKind::ConnectionAborted.into() });
                    }
                    move_me.push(*handle);
                }
            }
            for handle in move_me {
                if let Some(socket_handle) = self.listen_sockets.remove(&handle) {
                    self.tcp_sockets.insert(handle, socket_handle);
                }
            }

            for (handle, socket_handle) in self.dhcp_sockets.iter() {
                let socket = self.iface.get_socket::<Dhcpv4Socket>(*socket_handle);
                if let Some(evt) = socket.poll() {
                    drop(socket);
                    match evt {
                        Dhcpv4Event::Configured(config) => {
                            self.iface.update_ip_addrs(|target| {
                                if let ManagedSlice::Owned(vec) = target {
                                    if vec.iter().any(|cidr| cidr == &IpCidr::Ipv4(config.address)) == false {
                                        vec.push(IpCidr::Ipv4(config.address));
                                    }
                                }
                            });
                            self.iface.routes_mut().update(|target| {
                                if let ManagedMap::Owned(map) = target {
                                    if let Some(gw) = config.router.clone() {
                                        map.insert(
                                            IpCidr::Ipv4(Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 0)),
                                            Route::new_ipv4_gateway(gw));
                                    } else {
                                        map.clear();
                                    }
                                }
                            });
                            ret.push(PortResponse::DhcpConfigured {
                                handle: *handle,
                                address: wasmer_deploy_cli::model::IpCidr {
                                    ip: IpAddr::V4(config.address.address().into()),
                                    prefix: config.address.prefix_len(),
                                },
                                router: config.router.map(|a| IpAddr::V4(a.into())),
                                dns_servers: config.dns_servers
                                    .to_vec()
                                    .into_iter()
                                    .filter_map(|a| a.map(|a| IpAddr::V4(a.into())))
                                    .collect(),
                            });
                            ret.push(PortResponse::Nop { handle: *handle, ty: PortNopType::DhcpAcquire });
                        },
                        Dhcpv4Event::Deconfigured => {
                            self.iface.update_ip_addrs(|target| {
                                if let ManagedSlice::Owned(vec) = target {
                                    vec.clear();
                                }
                            });
                            ret.push(PortResponse::DhcpDeconfigured { handle: *handle });
                        }
                    }
                }
            }

            for (handle, socket_handle) in self.icmp_sockets.iter() {
                let socket = self.iface.get_socket::<IcmpSocket>(*socket_handle);
                while socket.can_recv() {
                    if let Ok((data, addr)) = socket.recv() {
                        let data = data.to_vec();
                        let peer_addr = conv_addr2(addr, 0);
                        ret.push(PortResponse::ReceivedFrom { handle: *handle, peer_addr, data });
                    } else {
                        break;
                    }
                }
            }
        }

        for (handle, raw) in self.raw_sockets.iter_mut() {
            while let Some(data) = raw.recv() {
                ret.push(PortResponse::Received { handle: handle.clone(), data })
            }
        }

        ret.append(&mut self.tx_queue);

        (ret, wait_time.into())
    }

    fn ip_mtu(&self) -> usize {
        self.iface.device().capabilities().ip_mtu()
    }

    fn ip_buf(&self, multiplier: usize) -> Vec<u8> {
        let mtu = self.ip_mtu() * multiplier;
        let mut ret = Vec::with_capacity(mtu);
        ret.resize_with(mtu, Default::default);
        ret
    }

    fn raw_mtu(&self) -> usize {
        self.iface.device().capabilities().max_transmission_unit
    }

    fn raw_buf(&self, multiplier: usize) -> Vec<u8> {
        let mtu = self.raw_mtu() * multiplier;
        let mut ret = Vec::with_capacity(mtu);
        ret.resize_with(mtu, Default::default);
        ret
    }
}

impl Drop
for Port
{
    fn drop(&mut self) {
        let tx = self.mac_drop.clone();
        let mac = HardwareAddress::from_bytes(self.mac.as_bytes());
        tokio::task::spawn(async move {
            let _ = tx.send(mac).await;
        });
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PortDevice {
    data: Arc<SegQueue<Vec<u8>>>,
    #[allow(dead_code)]
    #[derivative(Debug = "ignore")]
    mac: EthernetAddress,
    mtu: usize,
    switch: Arc<Switch>,
}

impl<'a> Device<'a> for PortDevice {
    type RxToken = RxToken;
    type TxToken = TxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        if let Some(buffer) = self.data.pop() {
            Some(
                (
                    RxToken {
                        buffer,
                    },
                    TxToken {
                        switch: self.switch.clone()
                    }
                )
            )
        } else {
            None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            switch: self.switch.clone()
        })

    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = self.mtu;
        caps.medium = Medium::Ethernet;
        caps.max_burst_size = None;
        caps.checksum = ChecksumCapabilities::default();
        caps
    }
}

#[doc(hidden)]
pub struct RxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        f(&mut self.buffer[..])
    }
}

#[doc(hidden)]
pub struct TxToken {
    switch: Arc<Switch>,
}

impl phy::TxToken for TxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buffer = vec![0; len];
        let result = f(&mut buffer);
        
        if result.is_ok() {
            self.switch.process(buffer, true, None);
        }

        result
    }
}

fn udp_meta_buf(buf_size: usize) -> Vec<UdpPacketMetadata> {
    let mut ret = Vec::with_capacity(buf_size);
    ret.resize_with(buf_size, || UdpPacketMetadata::EMPTY);
    ret
}

fn icmp_meta_buf(buf_size: usize) -> Vec<IcmpPacketMetadata> {
    let mut ret = Vec::with_capacity(buf_size);
    ret.resize_with(buf_size, || IcmpPacketMetadata::EMPTY);
    ret
}

fn conv_addr(addr: smoltcp::wire::IpEndpoint) -> SocketAddr {
    conv_addr2(addr.addr, addr.port)
}

fn conv_addr2(addr: smoltcp::wire::IpAddress, port: u16) -> SocketAddr {
    use smoltcp::wire::IpAddress::*;
    match addr {
        Ipv4(addr) => SocketAddr::V4(SocketAddrV4::new(addr.into(), port)),
        Ipv6(addr) => SocketAddr::V6(SocketAddrV6::new(addr.into(), port, 0 ,0)),
        _ => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
    }
}

fn conv_err(err: smoltcp::Error) -> SocketErrorKind {
    use smoltcp::Error::*;
    match err {
        Exhausted => SocketErrorKind::StorageFull,
        Illegal => SocketErrorKind::PermissionDenied,
        Unaddressable => SocketErrorKind::AddrNotAvailable,
        Finished => SocketErrorKind::ResourceBusy,
        Truncated => SocketErrorKind::InvalidData,
        Checksum => SocketErrorKind::InvalidData,
        Unrecognized => SocketErrorKind::InvalidData,
        Fragmented => SocketErrorKind::InvalidData,
        Malformed => SocketErrorKind::InvalidData,
        Dropped => SocketErrorKind::InvalidData,
        NotSupported => SocketErrorKind::Unsupported,
        _ => SocketErrorKind::Unsupported
    }
}

fn unicast_good(ip: IpAddr) -> bool {
    let ip: smoltcp::wire::IpAddress = ip.into();
    ip.is_unicast()
}

fn cidr_good(ip: IpAddr) -> bool {
    if let IpAddr::V4(ip) = ip {
        if ip.is_broadcast() ||
           ip.is_documentation() ||
           ip.is_link_local() ||
           ip.is_loopback() ||
           ip.is_multicast() ||
           ip.is_unspecified() {
            return false;
        }
    }
    if let IpAddr::V6(ip) = ip {
        if ip.is_loopback() ||
           ip.is_multicast() ||
           ip.is_unspecified() {
            return false;
        }
    }
    if ip.is_multicast() ||
       ip.is_unspecified() ||
       ip.is_loopback() {
        return false;
    }
    true
}
