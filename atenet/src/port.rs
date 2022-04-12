use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use smoltcp::iface::Routes;
use smoltcp::time::Instant;
use tokera::model::IpProtocol;
use tokera::model::IpVersion;
use tokio::sync::mpsc;
use derivative::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tokera::model::PortCommand;
use tokera::model::PortResponse;
use tokera::model::HardwareAddress;
use tokera::model::SocketHandle;
use tokera::model::SocketErrorKind;
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
use smoltcp::socket::RawSocket;
use smoltcp::socket::RawSocketBuffer;
use smoltcp::socket::RawPacketMetadata;
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
use smoltcp::wire::EthernetFrame;
use managed::ManagedSlice;
use managed::ManagedMap;

use super::switch::*;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Port
{
    pub(crate) switch: Arc<Switch>,
    pub(crate) raw_mode: bool,
    #[allow(dead_code)]
    #[derivative(Debug = "ignore")]
    pub(crate) mac: EthernetAddress,
    pub(crate) mac_drop: mpsc::Sender<HardwareAddress>,
    pub(crate) listen_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) tcp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) udp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) raw_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) icmp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) dhcp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    #[derivative(Debug = "ignore")]
    pub(crate) iface: Interface<'static, PortDevice>,
    pub(crate) buf_size: usize,
    pub(crate) errors: Vec<(SocketHandle, SocketErrorKind)>,
}

impl Port
{
    pub fn new(switch: &Arc<Switch>, mac: HardwareAddress, rx: mpsc::Receiver<Vec<u8>>, mac_drop: mpsc::Sender<HardwareAddress>) -> Port {
        let mac = EthernetAddress::from_bytes(mac.as_bytes());
        let device = PortDevice {
            rx, 
            mac,
            mtu: 1500,
            switch: Arc::clone(switch)
        };
        let iface = InterfaceBuilder::new(device, vec![])
            .hardware_addr(mac.into())
            .ip_addrs(Vec::new())
            .routes(Routes::new(BTreeMap::<IpCidr, Route>::new()))
            .random_seed(fastrand::u64(..))
            .finalize();
        Port {
            switch: Arc::clone(switch),
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
            errors: Vec::new(),
        }
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
                        self.errors.push((handle, conv_err(err)));
                    }
                } else if let Some(socket_handle) = self.raw_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<RawSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..]) {
                        self.errors.push((handle, conv_err(err)));
                    }
                } else if self.udp_sockets.contains_key(&handle) {
                    self.errors.push((handle, SocketErrorKind::Unsupported));
                } else if self.icmp_sockets.contains_key(&handle) {
                    self.errors.push((handle, SocketErrorKind::Unsupported));
                } else {
                    self.errors.push((handle, SocketErrorKind::NotConnected));
                }
            },
            PortCommand::SendTo {
                handle,
                data,
                addr,
            } => {
                if let Some(socket_handle) = self.udp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<UdpSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..], addr.into()) {
                        self.errors.push((handle, conv_err(err)));
                    }
                } else if let Some(socket_handle) = self.tcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..]) {
                        self.errors.push((handle, conv_err(err)));
                    }
                } else if let Some(socket_handle) = self.icmp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<IcmpSocket>(*socket_handle);
                    if let Err(err) = socket.send_slice(&data[..], addr.ip().into()) {
                        self.errors.push((handle, conv_err(err)));
                    }
                } else if self.raw_sockets.contains_key(&handle) {
                    self.errors.push((handle, SocketErrorKind::Unsupported));
                } else {
                    self.errors.push((handle, SocketErrorKind::NotConnected));
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
                if let Some(socket_handle) = self.raw_sockets.remove(&handle) {
                    self.iface.remove_socket(socket_handle);
                }
                if let Some(socket_handle) = self.dhcp_sockets.remove(&handle) {
                    self.iface.remove_socket(socket_handle);
                }
            },
            PortCommand::BindRaw {
                handle,
                ip_version,
                ip_protocol,
            } => {
                let rx_buffer = RawSocketBuffer::new(raw_meta_buf(self.buf_size), self.raw_buf(1));
                let tx_buffer = RawSocketBuffer::new(raw_meta_buf(self.buf_size), self.raw_buf(1));
                let socket = RawSocket::new(conv_ip_version(ip_version), conv_ip_protocol(ip_protocol), rx_buffer, tx_buffer);
                self.raw_sockets.insert(handle, self.iface.add_socket(socket));
            },
            PortCommand::BindIcmp {
                handle,
                local_addr,
                hop_limit,
            } => {
                let rx_buffer = IcmpSocketBuffer::new(icmp_meta_buf(self.buf_size), self.raw_buf(1));
                let tx_buffer = IcmpSocketBuffer::new(icmp_meta_buf(self.buf_size), self.raw_buf(1));
                let mut socket = IcmpSocket::new(rx_buffer, tx_buffer);
                socket.set_hop_limit(Some(hop_limit));
                if let Err(err) = socket.bind(IcmpEndpoint::Udp(local_addr.into())) {
                    self.errors.push((handle, conv_err(err)));
                } else {
                    self.icmp_sockets.insert(handle, self.iface.add_socket(socket));
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
            },
            PortCommand::DhcpReset {
                handle,
            } => {
                if let Some(socket_handle) = self.dhcp_sockets.get(&handle) {
                    let socket = self.iface.get_socket::<Dhcpv4Socket>(*socket_handle);
                    socket.reset();
                }
            },
            PortCommand::BindUdp {
                handle,
                local_addr,
                hop_limit,
            } => {
                let rx_buffer = UdpSocketBuffer::new(udp_meta_buf(self.buf_size), self.ip_buf(1));
                let tx_buffer = UdpSocketBuffer::new(udp_meta_buf(self.buf_size), self.ip_buf(1));
                let mut socket = UdpSocket::new(rx_buffer, tx_buffer);
                socket.set_hop_limit(Some(hop_limit));
                if let Err(err) = socket.bind(local_addr) {
                    self.errors.push((handle, conv_err(err)));
                } else {
                    self.udp_sockets.insert(handle, self.iface.add_socket(socket));
                }
            },
            PortCommand::ConnectTcp {
                handle,
                local_addr,
                peer_addr,
                hop_limit,
            } => {
                let rx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                let tx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                let mut socket = TcpSocket::new(rx_buffer, tx_buffer);
                socket.set_hop_limit(Some(hop_limit));
                let socket_handle = self.iface.add_socket(socket);
                let (socket, cx) = self.iface.get_socket_and_context::<TcpSocket>(socket_handle);
                if let Err(err) = socket.connect(cx, peer_addr, local_addr) {
                    self.errors.push((handle, conv_err(err)));
                }
            },
            PortCommand::Listen {
                handle,
                local_addr,
                hop_limit,
            } => {
                let rx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                let tx_buffer = TcpSocketBuffer::new(self.ip_buf(self.buf_size));
                let mut socket = TcpSocket::new(rx_buffer, tx_buffer);
                socket.set_hop_limit(Some(hop_limit));
                if let Err(err) = socket.listen(local_addr) {
                    self.errors.push((handle, conv_err(err)));
                } else {
                    self.listen_sockets.insert(handle, self.iface.add_socket(socket));
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
            PortCommand::SetIpAddresses {
                ips,
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
                                        let diff = d.signed_duration_since(tokera::model::Utc::now());
                                        Instant::from_micros(Instant::now().micros() + diff.num_microseconds().unwrap_or(0))
                                    }),
                                    expires_at: route.expires_at.map(|d| {
                                        let diff = d.signed_duration_since(tokera::model::Utc::now());
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
        }
        Ok(())
    }

    pub fn poll(&mut self) -> Vec<PortResponse> {
        let timestamp = Instant::now();
        let readiness = match self.iface.poll(timestamp) {
            Ok(a) => a,
            Err(e) => {
                debug!("poll error: {}", e);
                false
            }
        };

        if readiness == false {
            return Vec::new();
        }

        let mut ret = Vec::new();

        for (handle, socket_handle) in self.udp_sockets.iter() {
            let socket = self.iface.get_socket::<UdpSocket>(*socket_handle);
            while socket.can_recv() {
                if let Ok((data, addr)) = socket.recv() {
                    let data = data.to_vec();
                    let peer_addr = conv_addr(addr);
                    ret.push(PortResponse::ReceivedFrom { handle: *handle, peer_addr, data });
                }
            }
        }

        for (handle, socket_handle) in self.tcp_sockets.iter() {
            let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
            if socket.can_recv() {
                let mut data = Vec::new();
                while socket.can_recv() {
                    let _ = socket.recv(|d| {
                        data.extend_from_slice(d);
                        (d.len(), d.len())
                    });
                }
                if data.len() > 0 {
                    ret.push(PortResponse::Received { handle: *handle, data });
                }
            }
        }

        for (handle, socket_handle) in self.raw_sockets.iter() {
            let socket = self.iface.get_socket::<RawSocket>(*socket_handle);
            while socket.can_recv() {
                if let Ok(d) = socket.recv() {
                    let data = d.to_vec();
                    ret.push(PortResponse::Received { handle: *handle, data });
                }
            }
        }

        for (handle, socket_handle) in self.listen_sockets.iter() {
            let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
            while socket.is_listening() {
                if socket.is_active() {
                    let peer_addr = conv_addr(socket.remote_endpoint());
                    ret.push(PortResponse::TcpAccepted { handle: *handle, peer_addr });
                } else {
                    ret.push(PortResponse::SocketError { handle: *handle, error: SocketErrorKind::ConnectionAborted.into() });
                }
            }
        }

        for (handle, err) in self.errors.drain(..) {
            ret.push(PortResponse::SocketError { handle, error: err.into() });
        }

        ret
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
    rx: mpsc::Receiver<Vec<u8>>,
    #[derivative(Debug = "ignore")]
    mac: EthernetAddress,
    mtu: usize,
    switch: Arc<Switch>,
}

impl<'a> Device<'a> for PortDevice {
    type RxToken = RxToken;
    type TxToken = TxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.rx.try_recv()
            .ok()
            .map(|buffer| {
                (
                    RxToken {
                        buffer,
                    },
                    TxToken {
                        src: self.mac,
                        switch: self.switch.clone()
                    }
                )
            })
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            src: self.mac,
            switch: self.switch.clone()
        })

    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = self.mtu;
        caps.medium = Medium::Ethernet;
        caps.max_burst_size = None;
        caps.checksum = ChecksumCapabilities::ignored();
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
    src: EthernetAddress,
    switch: Arc<Switch>,
}

impl phy::TxToken for TxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buffer = vec![0; len];
        let result = f(&mut buffer);
        
        // This should use unicast for destination MAC's that are unicast - other
        // MAC addresses such as multicast and broadcast should use broadcast
        let frame = EthernetFrame::new_checked(&buffer[..])?;
        let dst = frame.dst_addr();
        if dst.is_unicast() {
            let _ = self.switch.unicast(&self.src, &dst, buffer, true);
        } else {
            let _ = self.switch.broadcast(&self.src, buffer);
        }

        result
    }
}

fn raw_meta_buf(buf_size: usize) -> Vec<RawPacketMetadata> {
    let mut ret = Vec::with_capacity(buf_size);
    ret.resize_with(buf_size, || RawPacketMetadata::EMPTY);
    ret
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
    let port = addr.port;
    use smoltcp::wire::IpAddress::*;
    match addr.addr {
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

fn conv_ip_protocol(a: IpProtocol) -> smoltcp::wire::IpProtocol {
    match a {
        IpProtocol::HopByHop => smoltcp::wire::IpProtocol::HopByHop,
        IpProtocol::Icmp => smoltcp::wire::IpProtocol::Icmp,
        IpProtocol::Igmp => smoltcp::wire::IpProtocol::Igmp,
        IpProtocol::Tcp => smoltcp::wire::IpProtocol::Tcp,
        IpProtocol::Udp => smoltcp::wire::IpProtocol::Udp,
        IpProtocol::Ipv6Route => smoltcp::wire::IpProtocol::Ipv6Route,
        IpProtocol::Ipv6Frag => smoltcp::wire::IpProtocol::Ipv6Frag,
        IpProtocol::Icmpv6 => smoltcp::wire::IpProtocol::Icmpv6,
        IpProtocol::Ipv6NoNxt => smoltcp::wire::IpProtocol::Ipv6NoNxt,
        IpProtocol::Ipv6Opts => smoltcp::wire::IpProtocol::Ipv6Opts,
        IpProtocol::Unknown(code) => smoltcp::wire::IpProtocol::Unknown(code),
    }
}

fn conv_ip_version(a: IpVersion) -> smoltcp::wire::IpVersion {
    match a {
        IpVersion::Ipv4 => smoltcp::wire::IpVersion::Ipv4,
        IpVersion::Ipv6 => smoltcp::wire::IpVersion::Ipv6,
    }
}