#[allow(unused_imports)]
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use smoltcp::time::Instant;
use tokio::sync::mpsc;
use derivative::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tokera::model::PortCommand;
use tokera::model::PortResponse;
use tokera::model::SocketHandle;
use tokera::model::SocketErrorKind;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::IpCidr;
use smoltcp::iface;
use smoltcp::iface::Interface;
use smoltcp::iface::InterfaceBuilder;
use smoltcp::phy;
use smoltcp::phy::Device;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::phy::Medium;
use smoltcp::phy::ChecksumCapabilities;
use smoltcp::socket::TcpSocket;
use smoltcp::socket::TcpSocketBuffer;
use smoltcp::socket::UdpSocket;
use smoltcp::socket::UdpSocketBuffer;
use smoltcp::socket::UdpPacketMetadata;
use managed::ManagedSlice;

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
    pub(crate) listen_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) tcp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    pub(crate) udp_sockets: HashMap<SocketHandle, iface::SocketHandle>,
    #[derivative(Debug = "ignore")]
    pub(crate) iface: Interface<'static, PortDevice>,
    pub(crate) buf_size: usize,
    pub(crate) errors: Vec<(SocketHandle, SocketErrorKind)>,
}

impl Port
{
    pub fn new(switch: &Arc<Switch>, mac: EthernetAddress, rx: mpsc::Receiver<Vec<u8>>) -> Port {
        let device = PortDevice {
            rx, 
            mac,
            mtu: 1500,
            switch: Arc::clone(switch)
        };
        let iface = InterfaceBuilder::new(device, vec![])
            .hardware_addr(mac.into())
            .ip_addrs(Vec::new())
            .random_seed(fastrand::u64(..))
            .finalize();
        Port {
            switch: Arc::clone(switch),
            raw_mode: false,
            mac,
            udp_sockets: Default::default(),
            tcp_sockets: Default::default(),
            listen_sockets: Default::default(),
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
            },
            PortCommand::BindUdp {
                handle,
                local_addr,
                hop_limit,
            } => {
                let rx_buffer = UdpSocketBuffer::new(meta_buf(self.buf_size), self.ip_buf(1));
                let tx_buffer = UdpSocketBuffer::new(meta_buf(self.buf_size), self.ip_buf(1));
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
                backlog: _,
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
                self.iface.set_hardware_addr(EthernetAddress::from_bytes(&mac).into());
            },
            PortCommand::SetIpAddresses {
                ips,
            } => {
                self.iface.update_ip_addrs(|target| {
                    // Anything that is not a unicast will cause a panic
                    let mut ips = ips.into_iter()
                        .filter(|(ip, _)| {
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
                        })
                        .map(|(ip, prefix_len)| {
                            IpCidr::new(ip.into(), prefix_len)
                        })
                        .collect();

                    // Replace the IP addresses
                    if let ManagedSlice::Owned(vec) = target {
                        vec.clear();
                        vec.append(&mut ips);
                    }
                });
            },
        }
        Ok(())
    }

    pub fn poll(&mut self) -> Vec<PortResponse> {
        let timestamp = Instant::now();
        match self.iface.poll(timestamp) {
            Ok(_) => {}
            Err(e) => {
                debug!("poll error: {}", e);
            }
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
            while socket.can_recv() {
                let mut data = [0u8; 2048];
                if let Ok(size) = socket.recv_slice(&mut data) {
                    let data = data[..size].to_vec();
                    ret.push(PortResponse::Received { handle: *handle, data });
                }
            }
        }

        for (handle, socket_handle) in self.listen_sockets.iter() {
            let socket = self.iface.get_socket::<TcpSocket>(*socket_handle);
            while socket.is_listening() {
                if socket.is_active() {
                    let local_addr = conv_addr(socket.local_endpoint());
                    let peer_addr = conv_addr(socket.remote_endpoint());
                    ret.push(PortResponse::TcpAccepted { handle: *handle, local_addr, peer_addr });
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
        let _ = self.switch.broadcast(&self.src, buffer);
        result
    }
}

fn meta_buf(buf_size: usize) -> Vec<UdpPacketMetadata> {
    let mut ret = Vec::with_capacity(buf_size);
    ret.resize_with(buf_size, || UdpPacketMetadata::EMPTY);
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