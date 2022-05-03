#![allow(unreachable_code)]
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Weak;
use std::ops::*;
use std::sync::MutexGuard;
use std::time::Duration;
use ate_files::prelude::FileAccessor;
use bytes::Bytes;
use smoltcp::phy::ChecksumCapabilities;
use smoltcp::wire::DhcpMessageType;
use smoltcp::wire::DhcpRepr;
use smoltcp::wire::EthernetRepr;
use smoltcp::wire::Ipv4Repr;
use smoltcp::wire::UdpRepr;
use tokio::sync::mpsc;
use tokio::sync::watch;
use smoltcp::wire::IpCidr;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::EthernetFrame;
use smoltcp::wire::EthernetProtocol;
use smoltcp::wire::ArpPacket;
use smoltcp::wire::ArpHardware;
use smoltcp::wire::ArpRepr;
use smoltcp::wire::ArpOperation;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpProtocol;
use smoltcp::wire::Ipv4Address;
use smoltcp::wire::Ipv4Packet;
use smoltcp::wire::Ipv6Address;
use smoltcp::wire::Ipv6Packet;
use smoltcp::wire::UdpPacket;
use smoltcp::wire::DhcpPacket;
use smoltcp::wire::DHCP_MAX_DNS_SERVER_COUNT;
use derivative::*;
use tokio::sync::RwLock;
use std::sync::Mutex;
use tokio::sync::broadcast;
use ate::prelude::*;
use ttl_cache::TtlCache;
use crossbeam::queue::SegQueue;
use tokera::model::MeshNode;
use tokera::model::HardwareAddress;
use tokera::model::ServiceInstance;
use tokera::model::DhcpReservation;
use tokera::model::INSTANCE_ROOT_ID;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::port::*;
use super::udp::*;
use super::gateway::*;

#[derive(Debug)]
pub enum Destination
{
    LocalSmoltcp(SwitchPortSmoltcp),
    LocalRaw(SwitchPortRaw),
    LocalDuel(SwitchPortSmoltcp, SwitchPortRaw),
    PeerSwitch(IpAddr)
}

#[derive(Debug)]
pub struct SwitchPortSmoltcp {
    pub(crate) data: Arc<SegQueue<Vec<u8>>>,
    pub(crate) wake: Arc<watch::Sender<()>>,
    #[allow(dead_code)]
    pub(crate) mac: EthernetAddress,
}

#[derive(Debug)]
pub struct SwitchPortRaw {
    pub(crate) raw: broadcast::Sender<Vec<u8>>,
    pub(crate) wake: Arc<watch::Sender<()>>,
    #[allow(dead_code)]
    pub(crate) mac: EthernetAddress,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DataPlane {
    pub(crate) cidrs: Vec<IpCidr>,
    pub(crate) ports: HashMap<EthernetAddress, Destination>,
    pub(crate) promiscuous: HashSet<EthernetAddress>,
    pub(crate) peers: HashMap<PrimaryKey, IpAddr>,
    #[derivative(Debug = "ignore")]
    pub(crate) arp_throttle: HashMap<Ipv4Address, chrono::DateTime<chrono::Utc>>,
    #[derivative(Debug = "ignore")]
    pub(crate) mac4: TtlCache<EthernetAddress, Ipv4Address>,
    #[derivative(Debug = "ignore")]
    pub(crate) ip4: TtlCache<Ipv4Address, EthernetAddress>,
    #[derivative(Debug = "ignore")]
    pub(crate) mac6: TtlCache<EthernetAddress, Ipv6Address>,
    #[derivative(Debug = "ignore")]
    pub(crate) ip6: TtlCache<Ipv6Address, EthernetAddress>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ControlPlane {
    pub(crate) inst: DaoMut<ServiceInstance>,
    pub(crate) me_node_id: PrimaryKey,
}

impl ControlPlane
{
    pub async fn me_node(&mut self) -> Option<DaoMut<MeshNode>>
    {
        let mut inst = self.inst.as_mut();
        let iter = inst.mesh_nodes
            .iter_mut().await
            .ok();
        drop(inst);
        iter
            .map(|nodes| {
                nodes
                    .filter(|a| a.key() == &self.me_node_id)
                    .next()
            })
            .flatten()
    }
}

pub struct DhcpMessage
{
    src_mac: EthernetAddress,
    gw_addr: Ipv4Address,
    requested_ip: Option<Ipv4Address>,
    message_type: DhcpMessageType,
    transaction_id: u32,
    switch: Weak<Switch>,
}

impl Destination
{
    pub fn send(&self, switch: &Switch, pck: Vec<u8>, allow_forward: bool) {
        match self {
            Destination::LocalSmoltcp(port) => {
                port.data.push(pck);
                let _ = port.wake.send(());
            },
            Destination::LocalRaw(port) => {
                let _ = port.raw.send(pck);
            },
            Destination::LocalDuel(port_smoltcp, port_raw) => {
                port_smoltcp.data.push(pck.clone());
                let _ = port_smoltcp.wake.send(());
                let _ = port_raw.raw.send(pck);
            },
            Destination::PeerSwitch(peer) => {
                if allow_forward {
                    let pck = switch.encrypt_packet(&pck[..]);
                    let _ = switch.udp.send(pck, peer.clone());
                }
            },
        }
    }

    pub fn is_local(&self) -> bool {
        match self {
            Destination::LocalSmoltcp(_) => true,
            Destination::LocalRaw(_) => true,
            Destination::LocalDuel(..) => true,
            Destination::PeerSwitch(_) => false,
        }
    }
}

#[derive(Debug)]
pub struct Switch
{
    pub(crate) id: u128,
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) udp: UdpPeerHandle,
    pub(crate) encrypt: EncryptKey,
    #[allow(dead_code)]
    pub(crate) accessor: Arc<FileAccessor>,
    pub(crate) data_plane: Mutex<DataPlane>,
    pub(crate) control_plane: RwLock<ControlPlane>,
    pub(crate) mac_drop: mpsc::Sender<HardwareAddress>,
    pub(crate) dhcp_msg: mpsc::Sender<DhcpMessage>,
    pub(crate) me_node_key: PrimaryKey,
    #[allow(dead_code)]
    pub(crate) me_node_addr: IpAddr,
    #[allow(dead_code)]
    pub(crate) gateway: Arc<Gateway>,
    pub(crate) access_tokens: Vec<String>,
}

impl Switch
{
    pub const MAC_SNOOP_MAX: usize = u16::MAX as usize;
    pub const MAC_SNOOP_TTL: Duration = Duration::from_secs(14400); // 4 hours (CISCO default)

    pub async fn new(accessor: Arc<FileAccessor>, cidrs: Vec<IpCidr>, udp: UdpPeerHandle, gateway: Arc<Gateway>) -> Result<Arc<Switch>, AteError> {
        let (inst, bus, me_node) = {
            let chain_dio = accessor.dio.clone().as_mut().await;
            
            let mut inst = chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;
            
            let me_node = {
                let mut inst = inst.as_mut();
                if let Some(existing) = inst
                    .mesh_nodes
                    .iter_mut()
                    .await?
                    .filter(|m| m.node_addr == udp.local_ip())
                    .next()
                {
                    let key = existing.key().clone();
                    debug!("deleting existing switch node id={} for {}", key, udp.local_ip());                    
                    chain_dio.delete(&key).await?;
                }
                debug!("creating new switch for {}", udp.local_ip());
                inst.mesh_nodes.push(MeshNode {
                    node_addr: udp.local_ip(),
                    switch_ports: Default::default(),
                    dhcp_reservation: Default::default(),
                    promiscuous: Default::default(),
                })?
            };
            chain_dio.commit().await?;

            let bus = inst.mesh_nodes.bus().await?;
            (inst, bus, me_node)
        };
        let id = inst.id;
        let name = inst.id_str();

        let encrypt_key = EncryptKey::from_seed_string(inst.subnet.network_token.clone(), KeySize::Bit128);
        
        let mut access_tokens = Vec::new();
        access_tokens.push(inst.subnet.network_token.clone());

        let (dhcp_msg_tx, dhcp_msg_rx) = mpsc::channel(100);
        let (mac_drop_tx, mac_drop_rx) = mpsc::channel(100);
        let switch = Arc::new(Switch {
            id,
            name,
            accessor,
            udp,
            encrypt: encrypt_key,
            me_node_key: me_node.key().clone(),
            me_node_addr: me_node.node_addr.clone(),
            data_plane: Mutex::new(
                DataPlane {
                    cidrs,
                    arp_throttle: Default::default(),
                    ports: Default::default(),
                    peers: Default::default(),
                    mac4: TtlCache::new(Self::MAC_SNOOP_MAX),
                    ip4: TtlCache::new(Self::MAC_SNOOP_MAX),
                    mac6: TtlCache::new(Self::MAC_SNOOP_MAX),
                    ip6: TtlCache::new(Self::MAC_SNOOP_MAX),
                    promiscuous: Default::default(),
                }
            ),
            control_plane: RwLock::new(
                ControlPlane {
                    inst,
                    me_node_id: me_node.key().clone(),
                }
            ),
            mac_drop: mac_drop_tx,
            dhcp_msg: dhcp_msg_tx,
            gateway,
            access_tokens,
        });

        {
            let switch = switch.clone();
            tokio::task::spawn(async move {
                switch.run(bus, mac_drop_rx, dhcp_msg_rx).await;
            });
        }

        Ok(switch)
    }

    pub async fn new_port(self: &Arc<Switch>) -> Result<Port, AteError> {
        let mac = HardwareAddress::new();
        let (tx_broadcast, _) = broadcast::channel(1000);
        let (tx_wake, rx_wake) = watch::channel(());
        let tx_wake = Arc::new(tx_wake);
        let data = Arc::new(SegQueue::new());
        let switch_port_smoltcp = SwitchPortSmoltcp {
            data: data.clone(),
            wake: tx_wake.clone(),
            mac: EthernetAddress::from_bytes(mac.as_bytes()),
        };
        let switch_port_raw = SwitchPortRaw {
            raw: tx_broadcast.clone(),
            wake: tx_wake.clone(),
            mac: EthernetAddress::from_bytes(mac.as_bytes()),
        };

        // Update the data plane so that it can start receiving data
        {
            let mut state = self.data_plane.lock().unwrap();
            state.ports.insert(EthernetAddress::from_bytes(mac.as_bytes()), Destination::LocalDuel(switch_port_smoltcp, switch_port_raw));
        }

        // Update the control plane so that others know that the port is here
        {
            let mut state = self.control_plane.write().await;
            let dio = state.inst.dio_mut();
            if let Some(mut me_node) = state.me_node().await {
                let mut me_node = me_node.as_mut();
                me_node.switch_ports.insert(mac);
            }
            dio.commit().await?;
        };

        let mac_drop = self.mac_drop.clone();
        Ok(
            Port::new(self, mac, data, rx_wake, mac_drop, tx_broadcast)
        )
    }

    pub async fn set_promiscuous(self: &Arc<Switch>, mac: HardwareAddress, promiscuous: bool) -> Result<(), AteError>
    {
        // Update the data plane
        {
            let mac = EthernetAddress::from_bytes(mac.as_bytes());
            let mut state = self.data_plane.lock().unwrap();
            if promiscuous {
                state.promiscuous.insert(mac.clone());
            } else {
                state.promiscuous.remove(&mac);
            }
        }

        // Update the control plane
        {
            let mut state = self.control_plane.write().await;
            let dio = state.inst.dio_mut();
            if let Some(mut me_node) = state.me_node().await {
                let mut me_node = me_node.as_mut();
                if promiscuous {
                    me_node.promiscuous.insert(mac.clone());
                } else {
                    me_node.promiscuous.remove(&mac);
                }
            }
            dio.commit().await?;
        };
        Ok(())
    }

    pub fn cidrs(&self) -> Vec<IpCidr>
    {
        let state = self.data_plane.lock().unwrap();
        state.cidrs.clone()
    }

    pub fn arps(self: &Arc<Switch>, pck: Vec<u8>) {
        let mut state = self.data_plane.lock().unwrap();
        self.__arps(&mut state, &pck[..]);
    }

    fn __arps(self: &Arc<Switch>, state: &mut MutexGuard<DataPlane>, pck: &[u8]) -> bool
    {
        // If its an ARP packet then maybe its for the gateway and
        // we should send a reply ARP
        if self.gateway.process_arp_reply(pck, self, state) == true {
            return true;
        }

        // If its a DHCP request then we should respond to it
        self.__dhcp_process(pck )
    }

    fn __dhcp_process(self: &Arc<Switch>, pck: &[u8]) -> bool
    {
        if let Ok(frame_mac) = EthernetFrame::new_checked(pck) {
            if frame_mac.ethertype() == EthernetProtocol::Ipv4
            {
                let src_mac = frame_mac.src_addr();
                if let Ok(frame_ip) = Ipv4Packet::new_checked(frame_mac.payload())
                {
                    if frame_ip.next_header() == IpProtocol::Udp {
                        if let Ok(frame_udp) = UdpPacket::new_checked(frame_ip.payload())
                        {
                            if frame_udp.dst_port() == 67 {
                                if let Ok(frame_dhcp) = DhcpPacket::new_checked(frame_udp.payload())
                                {
                                    if let Ok(frame_dhcp_repr) = DhcpRepr::parse(&frame_dhcp)
                                    {
                                        // Determine the gateway IP (which is also the DHCP server IP)
                                        let gw_addr = self.gateway.ips
                                            .iter()
                                            .filter_map(|ip| {
                                                match ip {
                                                    IpAddress::Ipv4(ip) => Some(ip.clone()),
                                                    _ => None
                                                }
                                            })
                                            .next()
                                            .unwrap_or(Ipv4Address::new(127, 0, 0, 1));

                                        // Pass the DHCP message on to be processed by
                                        // the asynchronous processing loop
                                        let _ = self.dhcp_msg.try_send(DhcpMessage {
                                            src_mac,
                                            gw_addr,
                                            requested_ip: frame_dhcp_repr.requested_ip,
                                            message_type: frame_dhcp_repr.message_type,
                                            transaction_id: frame_dhcp.transaction_id(),
                                            switch: Arc::downgrade(self),
                                        });
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    async fn allocate_ipv4(&self, mac: EthernetAddress) -> Option<Ipv4Address>
    {
        let mac = tokera::model::HardwareAddress::from_bytes(mac.as_bytes());
        let mac_str = hex::encode(mac.as_bytes()).to_uppercase();
        let cidrs = self.cidrs();

        let mut control_plane = self.control_plane.write().await;
        let dio = control_plane.inst.dio_mut();

        // Force a sync which is needed so we can handle the race conditions
        if let Err(err) = dio.chain().sync().await {
            warn!("failed to sync before doing a DHCP allocation - {}", err);
        }

        // First check if there is already an IP for this MAC
        let mut already4 = HashSet::new();
        if let Ok(nodes) = control_plane.inst.mesh_nodes.iter().await
        {
            // First build a list of all the IPs that are alread allocated and if we
            // find a node for this 
            let mut found = None;
            for node in nodes {
                for (k, v) in node.dhcp_reservation.iter() {
                    let ip: Ipv4Addr = v.addr4.clone().into();
                    if k == &mac_str && &control_plane.me_node_id == node.key() {
                        found = Some(ip)
                    } else {                    
                        already4.insert(ip);
                    }
                }
            }

            // We have found one an IP address
            if let Some(found) = found
            {
                // If we have a race condition (two nodes have allocated the same IP)
                // - in order to fix this we will deallocate our own reservation for
                // this IP but keep it blacklisted.
                if already4.contains(&found)
                {
                    if let Some(mut me_node) = control_plane.me_node().await {
                        let mut me_node = me_node.as_mut();
                        me_node.dhcp_reservation.remove(&mac_str);
                    }
                    if let Err(err) = dio.commit().await {
                        warn!("failed to remove double DHCP entry - {}", err);
                    }
                } else {
                    // Otherwise we are good to go
                    // Return the IP address to the caller
                    return Some(found.into());
                }
            }
        }

        // Loop through all the cidrs and find one that has a free IP address
        for cidr in cidrs {
            let range = match cidr.address() {
                IpAddress::Ipv4(_) => 32 - cidr.prefix_len(),
                IpAddress::Ipv6(_) => 128 - cidr.prefix_len(),
                _ => { continue; }
            };
            if range <= 0 {
                continue;
            }
            let mut range = 2u128.pow(range as u32);
            if range <= 3 { continue; }
            range -= 3;
            
            match cidr.address() {
                IpAddress::Ipv4(ip) => {
                    let start: Ipv4Addr = ip.into();
                    let mut start: u32 = start.into();
                    start += 2;
                    let end = start + (range as u32);
                    for ip in start..end {
                        let ip: Ipv4Addr = ip.into();
                        if already4.contains(&ip) {
                            continue;
                        }
                        if let Some(mut me_node) = control_plane.me_node().await {
                            let mut me_node = me_node.as_mut();
                            me_node.dhcp_reservation.insert(mac_str, DhcpReservation {
                                mac,
                                addr4: ip,
                                addr6: Vec::new(),
                            });
                        } else {
                            continue;
                        }
                        dio.commit().await.ok()?;
                        return Some(ip.into());
                    }
                },
                _ => { continue; }
            }
        }

        // How sad... we do not have any IP addresses left (cry...)
        None
    }

    async fn __tick(&self)
    {
        let subnet = {
            let control_plane = self.control_plane.read().await;
            control_plane.inst.subnet.clone()
        };

        let mut data_plane = self.data_plane.lock().unwrap();
        data_plane.cidrs = super::common::subnet_to_cidrs(&subnet);
    }

    pub fn arp_request(self: &Arc<Self>, src_mac: EthernetAddress, src_ip: Ipv4Address, dst_ip: Ipv4Address)
    {
        // Check its in the CIDR and the backoff window
        let mut data_plane = self.data_plane.lock().unwrap();
        for cidr in data_plane.cidrs.iter()
        {
            if cidr.contains_addr(&IpAddress::Ipv4(dst_ip))
            {
                // Check the throttle
                let now = chrono::Utc::now();
                let last_send = data_plane.arp_throttle.get(&dst_ip).map(|a| a.clone());
                if let Some(last_send) = last_send {
                    let diff = now - last_send;
                    if diff.num_seconds() < 5 {
                        return;
                    }
                }

                // Record it so we can throttle
                data_plane.arp_throttle.insert(dst_ip, now);

                // Send the ARP
                let arp_repr = ArpRepr::EthernetIpv4 {
                    operation: ArpOperation::Request,
                    source_hardware_addr: src_mac,
                    source_protocol_addr: src_ip,
                    target_hardware_addr: EthernetAddress::BROADCAST,
                    target_protocol_addr: dst_ip,
                };
                let mut arp_bytes = vec![0xff; arp_repr.buffer_len()];
                let mut arp_packet = ArpPacket::new_unchecked(&mut arp_bytes[..]);
                arp_repr.emit(&mut arp_packet);

                // Build the Ethernet payload
                let eth_repr = EthernetRepr {
                    src_addr: src_mac,
                    dst_addr: EthernetAddress::BROADCAST,
                    ethertype: EthernetProtocol::Arp,
                };
                let mut eth_bytes = vec![0x00; eth_repr.buffer_len() + arp_repr.buffer_len()];
                let mut eth_packet = EthernetFrame::new_unchecked(&mut eth_bytes[..]);
                eth_repr.emit(&mut eth_packet);
                eth_packet.payload_mut().copy_from_slice(&arp_bytes[..]);

                #[cfg(feature="tcpdump")]
                tcpdump(self.me_node_addr, self.name.as_str(), "BROADCAST", &eth_bytes[..]);

                // Broadcast it
                self.__broadcast(&mut data_plane, &src_mac, &eth_bytes[..], true, None);
                return;
            }
        }
    }

    async fn __dhcp_process_internal(self: &Arc<Switch>, msg: DhcpMessage)
    {
        // Determine the IP address for this particular MAC address
        let client_ip = self.allocate_ipv4(msg.src_mac).await;
        let is_decline = client_ip.is_none();
        let client_ip = client_ip
            .unwrap_or(Ipv4Address::UNSPECIFIED);

        // Compute the subnet mask
        let subnet = if client_ip.is_unicast() {
            self.cidrs()
                .iter()
                .filter(|cidr| {
                    let client_ip: Ipv4Address = client_ip.into();
                    cidr.contains_addr(&IpAddress::Ipv4(client_ip))
                })
                .map(|cidr| {
                    let prefix = 32u32 - (cidr.prefix_len() as u32);
                    if prefix <= 1 {
                        Ipv4Address::new(255, 255, 255, 255)
                    } else {
                        let mask = 2u32.pow(prefix) - 1u32;
                        let mask = mask ^ u32::MAX;
                        let mask: Ipv4Addr = mask.into();
                        mask.into()
                    }
                })
                .next()
                .unwrap_or(Ipv4Address::new(255, 255, 255, 255))
        } else {
            Ipv4Address::UNSPECIFIED
        };

        // Determine the DNS servers
        let mut dns_servers = [None; DHCP_MAX_DNS_SERVER_COUNT];
        dns_servers[0] = Some(Ipv4Address::new(8, 8, 8, 8));
    
        // Build the DHCP datagram
        let dhcp_repr = DhcpRepr {
            message_type: match is_decline {
                true => DhcpMessageType::Nak,
                false => match msg.message_type {
                    DhcpMessageType::Discover => DhcpMessageType::Offer,
                    DhcpMessageType::Request => {
                        match msg.requested_ip {
                            Some(ip) if ip == client_ip => DhcpMessageType::Ack,
                            _ => DhcpMessageType::Nak
                        }
                    },
                    _ => {
                        return;
                    }
                }
            },
            transaction_id: msg.transaction_id,
            client_hardware_address: msg.src_mac,
            client_ip: client_ip,
            your_ip: client_ip,
            server_ip: msg.gw_addr,
            router: Some(msg.gw_addr),
            subnet_mask: Some(subnet),
            relay_agent_ip: Ipv4Address::UNSPECIFIED,
            broadcast: false,
            requested_ip: msg.requested_ip,
            client_identifier: Some(msg.src_mac),
            server_identifier: Some(msg.gw_addr),
            parameter_request_list: None,
            dns_servers: Some(dns_servers),
            max_size: None,
            lease_duration: Some(0xffff_ffff), // Infinite lease
        };
        let mut dhcp_payload = vec![0xa5; dhcp_repr.buffer_len()];
        let mut dhcp_packet = DhcpPacket::new_unchecked(&mut dhcp_payload);
        dhcp_repr.emit(&mut dhcp_packet).unwrap();

        // Set the IP addresses
        let src_addr = msg.gw_addr;
        let dst_addr = Ipv4Address::BROADCAST;

        // Build the UDP payload
        let udp_repr = UdpRepr {
            src_port: smoltcp::wire::DHCP_SERVER_PORT,
            dst_port: smoltcp::wire::DHCP_CLIENT_PORT,
        };
        let mut udp_bytes = vec![0xff; udp_repr.header_len() + dhcp_payload.len()];
        let mut udp_packet = UdpPacket::new_unchecked(&mut udp_bytes[..]);
        udp_repr.emit(
            &mut udp_packet,
            &IpAddress::Ipv4(msg.gw_addr),
            &IpAddress::Ipv4(Ipv4Address::new(255, 255, 255, 255)),
            dhcp_payload.len(),
            |buf| buf.copy_from_slice(&dhcp_payload[..]),
            &ChecksumCapabilities::default(),
        );
        udp_packet.fill_checksum(&IpAddress::Ipv4(src_addr), &IpAddress::Ipv4(dst_addr));
        
        // Build the IPv4 payload
        let ipv4_repr = Ipv4Repr {
            src_addr,
            dst_addr,
            next_header: IpProtocol::Udp,
            payload_len: udp_bytes.len(),
            hop_limit: 64,
        };
        let mut ip_bytes = vec![0xa5; ipv4_repr.buffer_len() + udp_bytes.len()];
        let mut ip_packet = Ipv4Packet::new_unchecked(&mut ip_bytes[..]);
        ipv4_repr.emit(&mut ip_packet, &ChecksumCapabilities::default());
        ip_packet.payload_mut().copy_from_slice(&udp_bytes[..]);
        ip_packet.fill_checksum();

        // Build the Ethernet payload
        let eth_repr = EthernetRepr {
            src_addr: Gateway::MAC,
            dst_addr: msg.src_mac,
            ethertype: EthernetProtocol::Ipv4,
        };
        let mut eth_bytes = vec![0x00; eth_repr.buffer_len() + ip_bytes.len()];
        let mut eth_packet = EthernetFrame::new_unchecked(&mut eth_bytes[..]);
        eth_repr.emit(&mut eth_packet);
        eth_packet.payload_mut().copy_from_slice(&ip_bytes[..]);

        // Send the response to the caller
        self.unicast(&Gateway::MAC, &msg.src_mac, eth_bytes, false, None);
    }

    pub fn broadcast(self: &Arc<Switch>, src: &EthernetAddress, pck: Vec<u8>, allow_forward: bool, set_peer: Option<&IpAddr>) {
        #[cfg(feature="tcpdump")]
        tcpdump(self.me_node_addr, self.name.as_str(), "BROADCAST", &pck[..]);

        let mut state = self.data_plane.lock().unwrap();
        self.__broadcast(&mut state, src, &pck[..], allow_forward, set_peer);
    }
    
    fn __broadcast(self: &Arc<Switch>, state: &mut MutexGuard<DataPlane>, src: &EthernetAddress, pck: &[u8], allow_forward: bool, set_peer: Option<&IpAddr>)
    {
        // If its an ARP packet then maybe its for the gateway and
        // we should send a reply ARP
        self.gateway.process_arp_reply(pck, self, state);

        // Process the packet
        for (mac, dst) in state.ports.iter() {
            let pck = pck.to_vec();
            if src != mac {
                dst.send(self, pck, false);
            }
        }

        // Only if we allow forwarding
        if allow_forward {
            self.__broadcast_to_peers(state, pck);
        }

        // Snoop the packet
        self.snoop(state, &pck[..], set_peer);
    }

    fn __broadcast_to_peers(self: &Arc<Switch>, state: &mut MutexGuard<DataPlane>, pck: &[u8])
    {
        // Encrypt and sign the packet before we send it
        let pck = self.encrypt_packet(pck);
        for peer in state.peers.values() {
            let pck = pck.clone();
            let _ = self.udp.send(pck, peer.clone());
        }
    }

    pub fn broadcast_and_arps(self: &Arc<Switch>, src: &EthernetAddress, pck: Vec<u8>, allow_forward: bool, set_peer: Option<&IpAddr>) {
        #[cfg(feature="tcpdump")]
        tcpdump(self.me_node_addr, self.name.as_str(), "BROADCAST", &pck[..]);

        let mut state = self.data_plane.lock().unwrap();
        if self.__arps(&mut state, &pck[..]) == false {
            self.__broadcast(&mut state, src, &pck[..], allow_forward, set_peer);
        }
    }

    pub fn unicast(self: &Arc<Switch>, src: &EthernetAddress, dst_mac: &EthernetAddress, pck: Vec<u8>, allow_forward: bool, set_peer: Option<&IpAddr>)
    {
        #[cfg(feature="tcpdump")]
        tcpdump(self.me_node_addr, self.name.as_str(), "UNICAST  ", &pck[..]);

        // If the packet is going to the default gateway then we
        // should pass it to our dateway engine to process instead
        if dst_mac == &Gateway::MAC
        {
            {
                // We should snoop all the packets
                let mut state = self.data_plane.lock().unwrap();
                self.snoop(&mut state, &pck[..], None);
                
                // There are certain situations where we forward to the peers (namely if an ARP
                // reply goes back to the gateway - this is so that all the gateways can update
                // there internal state)
                if allow_forward
                {
                    // If this is an ARP packet then we should broadcast it
                    if let Ok(frame_mac) = EthernetFrame::new_checked(&pck[..]) {
                        if frame_mac.ethertype() == EthernetProtocol::Arp {
                            self.__broadcast_to_peers(&mut state, &pck[..]);
                        }
                    }
                }

                // Promiscuous devices might also be listening to the gateway address
                if state.promiscuous.is_empty() {
                    self.__promiscuous(&mut state, dst_mac, &pck[..], allow_forward);
                }
            }

            // Process the outbound packet which will do some IP routing
            self.gateway.process_outbound(pck);
            return;
        }

        let mut state = self.data_plane.lock().unwrap();
        self.__unicast(&mut state, src, dst_mac, pck, allow_forward, set_peer);
    }

    pub(crate) fn __unicast(self: &Arc<Switch>, state: &mut MutexGuard<DataPlane>, src: &EthernetAddress, dst_mac: &EthernetAddress, pck: Vec<u8>, allow_forward: bool, set_peer: Option<&IpAddr>)
    {
        // We might need to snoop this packet
        self.snoop(state, &pck[..], set_peer);

        // If the destination is known to us
        if state.ports.contains_key(&dst_mac)
        {
            // We might need to make a copy of the packet so that any promiscuous
            // ports can also get a copy
            if state.promiscuous.is_empty() == false {
                self.__promiscuous(state, dst_mac, &pck[..], allow_forward);
            }

            // Send the packet to the destination
            if let Some(dst) = state.ports.get(&dst_mac) {
                dst.send(self, pck, allow_forward);
            }
        } else {
            // Otherwise we broadcast it to all the other nodes as it
            // could be that we just dont know about it yet or it could
            // be that its multicast/broadcast traffic.
            self.__broadcast(state, src, &pck[..], allow_forward, set_peer);
        }
    }

    pub(crate) fn __promiscuous(self: &Arc<Switch>, state: &mut MutexGuard<DataPlane>, dst_mac: &EthernetAddress, pck: &[u8], allow_forward: bool)
    {
        if state.promiscuous.is_empty() {
            return;
        }

        let mut peers = HashSet::new();
        for mac in state.promiscuous.iter() {
            if mac == dst_mac {
                continue;
            }
            match state.ports.get(mac) {
                Some(Destination::LocalSmoltcp(port)) => {
                    port.data.push(pck.to_vec());
                    let _ = port.wake.send(());
                },
                Some(Destination::LocalRaw(port)) => {
                    let _ = port.raw.send(pck.to_vec());
                },
                Some(Destination::LocalDuel(port_smoltcp, port_raw)) => {
                    port_smoltcp.data.push(pck.to_vec());
                    let _ = port_smoltcp.wake.send(());
                    let _ = port_raw.raw.send(pck.to_vec());
                },
                Some(Destination::PeerSwitch(peer)) => {
                    peers.insert(peer.clone());
                },
                _ => { }
            }
        }

        // We do not send the packet twice
        if allow_forward && peers.len() > 0 {
            if let Some(Destination::PeerSwitch(peer)) = state.ports.get(dst_mac) {
                peers.remove(peer);
            }
            for peer in peers {
                let pck = self.encrypt_packet(pck);
                let _ = self.udp.send(pck, peer.clone());
            }
        }
    }

    pub fn snoop(&self, state: &mut MutexGuard<DataPlane>, pck: &[u8], set_peer: Option<&IpAddr>) {
        if let Ok(frame_mac) = EthernetFrame::new_checked(&pck[..]) {
            let mac = frame_mac.src_addr();
            match frame_mac.ethertype() {
                EthernetProtocol::Ipv4 => {
                    if let Ok(frame_ip) = Ipv4Packet::new_checked(frame_mac.payload()) {
                        let ip = frame_ip.src_addr();

                        if mac.is_unicast() &&
                           ip.is_unicast()
                        {
                            let update_mac4 = state.mac4.contains_key(&mac) == false;
                            let update_ip4 = state.ip4.contains_key(&ip) == false;
                            
                            if update_mac4 || update_ip4 {
                                state.mac4.insert(mac, ip, Self::MAC_SNOOP_TTL);
                                state.ip4.insert(ip, mac, Self::MAC_SNOOP_TTL);

                                if let Some(set_peer) = set_peer {
                                    state.ports.insert(mac, Destination::PeerSwitch(set_peer.clone()));
                                }
                            }
                        }
                    }
                },
                EthernetProtocol::Ipv6 => {
                    if let Ok(frame_ip) = Ipv6Packet::new_checked(frame_mac.payload()) {
                        let ip = frame_ip.src_addr();

                        if mac.is_unicast() &&
                           ip.is_unicast()
                        {
                            let update_mac6 = state.mac6.contains_key(&mac) == false;
                            let update_ip6 = state.ip6.contains_key(&ip) == false;
                            
                            if update_mac6 || update_ip6 {
                                state.mac6.insert(mac, ip, Self::MAC_SNOOP_TTL);
                                state.ip6.insert(ip, mac, Self::MAC_SNOOP_TTL);

                                if let Some(set_peer) = set_peer {
                                    state.ports.insert(mac, Destination::PeerSwitch(set_peer.clone()));
                                }
                            }
                        }
                    }
                },
                EthernetProtocol::Arp => {
                    if let Ok(frame_arp) = ArpPacket::new_checked(frame_mac.payload()) {
                        if frame_arp.hardware_type() == ArpHardware::Ethernet &&
                            frame_arp.protocol_type() == EthernetProtocol::Ipv4
                        {
                            let mac = EthernetAddress::from_bytes(frame_arp.source_hardware_addr());
                            let ip = Ipv4Address::from_bytes(frame_arp.source_protocol_addr());

                            if mac != EthernetAddress::BROADCAST &&
                               mac != Gateway::MAC &&
                               ip != Ipv4Address::BROADCAST &&
                               ip != Ipv4Address::UNSPECIFIED
                            {
                                state.mac4.insert(mac, ip, Self::MAC_SNOOP_TTL);
                                state.ip4.insert(ip, mac, Self::MAC_SNOOP_TTL);

                                if let Some(set_peer) = set_peer {
                                    state.ports.insert(mac, Destination::PeerSwitch(set_peer.clone()));
                                }
                            }
                        }
                    }
                },
                _ => { }
            }
        }
    }

    pub fn lookup_ip(&self, ip: &IpAddress) -> Option<EthernetAddress>
    {
        match ip {
            IpAddress::Ipv4(ip) => self.lookup_ipv4(ip),
            IpAddress::Ipv6(ip) => self.lookup_ipv6(ip),
            _ => None
        }
    }

    pub fn lookup_ipv4(&self, ip: &Ipv4Address) -> Option<EthernetAddress>
    {
        let state = self.data_plane.lock().unwrap();
        state.ip4.get(ip).map(|mac| mac.clone())
    }

    pub fn lookup_ipv6(&self, ip: &Ipv6Address) -> Option<EthernetAddress>
    {
        let state = self.data_plane.lock().unwrap();
        state.ip6.get(ip).map(|mac| mac.clone())
    }

    pub fn encrypt_packet(&self, pck: &[u8]) -> Bytes {
        let prefix = self.id.to_be_bytes();
        let hash = AteHash::from_bytes(&pck[..]);
        let capacity = prefix.len() + pck.len() + hash.len();
        let pck = self.encrypt.encrypt_with_hash_iv_with_capacity_and_prefix(&hash, &pck[..], capacity, &prefix);
        Bytes::from(pck)
    }

    pub fn decrypt_packet(&self, data: &[u8], hash: AteHash) -> Option<Vec<u8>> {
        let pck = self.encrypt.decrypt_with_hash_iv(&hash, data);
        let test = AteHash::from_bytes(&pck[..]);
        if test == hash {
            Some(pck)
        } else {
            debug!("packet dropped - invalid hash {} vs {}", test, hash);
            None
        }
    }

    pub fn process_peer_packet(self: &Arc<Switch>, pck: &[u8], hash: AteHash, peer: IpAddr) {
        if let Some(pck) = self.decrypt_packet(pck, hash) {
            // This should use unicast for destination MAC's that are unicast - other
            // MAC addresses such as multicast and broadcast should use broadcast
            match EthernetFrame::new_checked(&pck[..]) {
                Ok(frame) => {
                    let src = frame.src_addr();
                    let dst = frame.dst_addr();
                    if dst.is_unicast() {
                        let _ = self.unicast(&src, &dst, pck, false, Some(&peer));
                    } else {
                        let _ = self.broadcast(&src, pck, false, Some(&peer));
                    }
                }
                Err(err) => {
                    debug!("packet dropped - {}", err);
                }
            }
        }
    }

    pub fn has_access(&self, access_token: &str) -> bool {
        self.access_tokens
            .iter()
            .any(|a| a == access_token)
    }

    pub async fn remove_node(&self, node_key: &PrimaryKey)
    {
        if node_key == &self.me_node_key {
            return;
        }
        info!("switch node deleted (id={}, key={})", self.id, node_key);

        let mut prom_remove = Vec::new();
        let mut state = self.data_plane.lock().unwrap();
        if let Some(node_addr) = state.peers.remove(node_key) {
            state.ports.retain(|m, v| {
                match v {
                    Destination::PeerSwitch(s) => {
                        if s == &node_addr {
                            prom_remove.push(m.clone());
                            false
                        } else { true }
                    },
                    _ => true
                }
            });
        }
        prom_remove.into_iter().for_each(|m| {
            state.promiscuous.remove(&m);
        });
    }

    pub async fn update_node(&self, node_key: &PrimaryKey, node: &MeshNode)
    {
        if node_key == &self.me_node_key {
            return;
        }
        debug!("switch node updated (id={}, node_addr={})", self.id, node.node_addr);

        let mut prom_remove = Vec::new();
        let mut state = self.data_plane.lock().unwrap();
        state.ports.retain(|m, v| {
            match v {
                Destination::PeerSwitch(s) => {
                    if s == &node.node_addr {
                        prom_remove.push(m.clone());
                        false
                    } else { true }
                },
                _ => true
            }
        });
        prom_remove.into_iter().for_each(|m| {
            state.promiscuous.remove(&m);
        });
        debug!("adding switch peer (node_key={}, addr={})", node_key, node.node_addr);
        state.peers.insert(node_key.clone(), node.node_addr);
        for mac in node.switch_ports.iter() {
            let mac = EthernetAddress::from_bytes(mac.as_bytes());
            debug!("adding switch port (mac={}, addr={})", mac, node.node_addr);
            state.ports.insert(mac, Destination::PeerSwitch(node.node_addr));
        }
        for mac in node.promiscuous.iter() {
            let mac = EthernetAddress::from_bytes(mac.as_bytes());
            debug!("promiscuous switch port (mac={})", mac);
            state.promiscuous.insert(mac);
        }
    }

    pub async fn run(&self, mut bus: Bus<MeshNode>, mut mac_drop: mpsc::Receiver<HardwareAddress>, mut dhcp_msg_rx: mpsc::Receiver<DhcpMessage>)
    {
        debug!("control thread initializing");

        // We first do a full update in this background thread
        // to prevent race conditions missing the updates
        {
            let state = self.control_plane.read().await;
            self.gateway.update(state.inst.subnet.peerings.clone()).await;
            for node in state.inst.mesh_nodes.iter().await.unwrap() {
                self.update_node(node.key(), node.deref()).await;
            }
        }

        debug!("control thread running");

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(20));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.__tick().await;
                },
                evt = bus.recv() => {
                    match evt {
                        Ok(evt) => {
                            match evt {
                                BusEvent::Updated(node) => {
                                    self.update_node(node.key(), node.deref()).await;
                                },
                                BusEvent::Deleted(key) => {
                                    self.remove_node(&key).await;
                                },
                            }
                        }
                        Err(err) => {
                            warn!("control thread closing (1) - {:?}", err);
                            break;
                        }
                    }
                },
                mac = mac_drop.recv() => {
                    if let Some(mac) = mac {
                        let mut state = self.control_plane.write().await;
                        let dio = state.inst.dio_mut();
                        if let Some(mut me_node) = state.me_node().await {
                            let mut me_node = me_node.as_mut();
                            me_node.switch_ports.remove(&mac);
                            me_node.promiscuous.remove(&mac);

                            let mac_str = hex::encode(mac.as_bytes()).to_uppercase();
                            me_node.dhcp_reservation.remove(&mac_str);
                        }
                        let _ = dio.commit().await;
                    } else {
                        debug!("control thread closing (2)");
                        break;
                    }
                },
                msg = dhcp_msg_rx.recv() => {
                    if let Some(msg) = msg {
                        if let Some(switch) = msg.switch.upgrade() {
                            switch.__dhcp_process_internal(msg).await;
                        }
                    } else {
                        debug!("control thread closing (3)");
                        break;
                    }
                }
            }
        }

        // Clear the data plane as we are going offline
        {
            let mut state = self.data_plane.lock().unwrap();
            state.peers.clear();
            state.ports.clear();
        }

        // Need to remove the node from the switch in the control plane
        let state = self.control_plane.write().await;
        let dio = state.inst.dio_mut();
        if dio.delete(&state.me_node_id).await.is_ok() {
            let _ = dio.commit().await;
        }

        debug!("control thread exited");
    }
}

#[cfg(feature="tcpdump")]
fn tcpdump(node_ip: IpAddr, sw: &str, ty: &str, pck: &[u8])
{
    let pck = smoltcp::wire::PrettyPrinter::<EthernetFrame<&[u8]>>::new("", &pck);
    info!("{}@{}: {} {}", &sw[..4], node_ip, ty, pck);
}