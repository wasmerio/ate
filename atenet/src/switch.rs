#![allow(unreachable_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::ops::*;
use std::sync::RwLockReadGuard;
use std::time::Duration;
use ate_files::prelude::FileAccessor;
use tokio::sync::mpsc;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::EthernetFrame;
use smoltcp::wire::EthernetProtocol;
use smoltcp::wire::ArpPacket;
use smoltcp::wire::ArpHardware;
use smoltcp::wire::IpAddress;
use smoltcp::wire::Ipv4Address;
use smoltcp::wire::Ipv4Packet;
use smoltcp::wire::Ipv6Address;
use smoltcp::wire::Ipv6Packet;
use derivative::*;
use tokio::sync::RwLock;
use std::sync::RwLock as StdRwLock;
use ate::prelude::*;
use ttl_cache::TtlCache;
use tokera::model::MeshNode;
use tokera::model::HardwareAddress;
use tokera::model::ServiceInstance;
use tokera::model::INSTANCE_ROOT_ID;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::port::*;
use super::common::*;
use super::udp::*;
use super::gateway::*;

#[derive(Debug)]
pub enum Destination
{
    Local(SwitchPort),
    PeerSwitch(IpAddr)
}

#[derive(Debug)]
pub struct SwitchPort {
    tx: mpsc::Sender<Vec<u8>>,
    #[allow(dead_code)]
    mac: EthernetAddress,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DataPlane {
    pub(crate) ports: HashMap<EthernetAddress, Destination>,
    pub(crate) peers: HashMap<PrimaryKey, IpAddr>,
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
    pub(crate) me_node: DaoMut<MeshNode>,
}

#[derive(Debug)]
pub struct Switch
{
    pub(crate) id: u128,
    pub(crate) udp: UdpPeer,
    pub(crate) encrypt: EncryptKey,
    #[allow(dead_code)]
    pub(crate) accessor: Arc<FileAccessor>,
    pub(crate) data_plane: StdRwLock<DataPlane>,
    pub(crate) control_plane: RwLock<ControlPlane>,
    pub(crate) mac_drop: mpsc::Sender<HardwareAddress>,
    pub(crate) me_node_key: PrimaryKey,
    #[allow(dead_code)]
    pub(crate) gateway: Arc<Gateway>,
}

impl Switch
{
    pub const MAC_SNOOP_MAX: usize = u16::MAX as usize;
    pub const MAC_SNOOP_TTL: Duration = Duration::from_secs(14400); // 4 hours (CISCO default)

    pub async fn new(accessor: Arc<FileAccessor>, udp: UdpPeer, gateway: Arc<Gateway>) -> Result<Arc<Switch>, AteError> {
        let (inst, bus, me_node) = {
            let chain_dio = accessor.dio.clone().as_mut().await;
            
            let mut inst = chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;
            
            let me_node = {
                let mut inst = inst.as_mut();
                match inst
                    .mesh_nodes
                    .iter_mut()
                    .await?
                    .filter(|m| m.node_addr == udp.local_ip())
                    .next()
                {
                    Some(mut a) => {
                        {
                            let mut a = a.as_mut();
                            a.switch_ports.clear();
                        }
                        a
                    },
                    None => {
                        inst.mesh_nodes.push(MeshNode {
                            node_addr: udp.local_ip(),
                            switch_ports: Default::default(),
                        })?
                    }
                }
            };
            chain_dio.commit().await?;

            let bus = inst.mesh_nodes.bus().await?;
            (inst, bus, me_node)
        };
        let id = inst.id;

        let encrypt_key = EncryptKey::from_seed_string(inst.subnet.network_token.clone(), KeySize::Bit128);

        let (mac_drop_tx, mac_drop_rx) = mpsc::channel(100);
        let switch = Arc::new(Switch {
            id,
            accessor,
            udp,
            encrypt: encrypt_key,
            me_node_key: me_node.key().clone(),
            data_plane: StdRwLock::new(
                DataPlane {
                    ports: Default::default(),
                    peers: Default::default(),
                    mac4: TtlCache::new(Self::MAC_SNOOP_MAX),
                    ip4: TtlCache::new(Self::MAC_SNOOP_MAX),
                    mac6: TtlCache::new(Self::MAC_SNOOP_MAX),
                    ip6: TtlCache::new(Self::MAC_SNOOP_MAX),
                }
            ),
            control_plane: RwLock::new(
                ControlPlane {
                    inst,
                    me_node,
                }
            ),
            mac_drop: mac_drop_tx,
            gateway,
        });

        {
            let switch = switch.clone();
            tokio::task::spawn(async move {
                switch.run(bus, mac_drop_rx).await;
            });
        }

        Ok(switch)
    }

    pub async fn new_port(self: &Arc<Switch>) -> Result<Port, AteError> {
        let mac = HardwareAddress::new();
        let (tx, rx) = mpsc::channel(MAX_MPSC);
        let switch_port = SwitchPort {
            tx,
            mac: EthernetAddress::from_bytes(mac.as_bytes()),
        };

        // Update the data plane so that it can start receiving data
        {
            let mut state = self.data_plane.write().unwrap();
            state.ports.insert(EthernetAddress::from_bytes(mac.as_bytes()), Destination::Local(switch_port));
        }

        // Update the control plane so that others know that the port is here
        {
            let mut state = self.control_plane.write().await;
            let dio = state.me_node.dio_mut();
            {
                let mut me_node = state.me_node.as_mut();
                me_node.switch_ports.insert(mac);
            }
            dio.commit().await?;
        };

        let mac_drop = self.mac_drop.clone();
        Ok(
            Port::new(self, mac, rx, mac_drop)
        )
    }

    pub fn broadcast(&self, src: &EthernetAddress, pck: Vec<u8>) {
        let state = self.data_plane.read().unwrap();
        self.__broadcast(&state, src, &pck[..]);
    }
    
    fn __broadcast(&self, state: &RwLockReadGuard<DataPlane>, src: &EthernetAddress, pck: &[u8]) {
        for (mac, dst) in state.ports.iter() {
            if let Destination::Local(port) = dst {
                if src != mac {
                    let _ = port.tx.blocking_send(pck.to_vec());
                }
            }
        }

        // Encrypt and sign the packet before we send it
        let pck = self.encrypt_packet(pck);
        for peer in state.peers.values() {
            self.udp.send(&pck[..], peer.clone());
        }
    }

    pub fn unicast(&self, src: &EthernetAddress, dst_mac: &EthernetAddress, pck: Vec<u8>, allow_forward: bool)
    {
        // If the packet is going to the default gateway then we
        // should pass it to our dateway engine to process instead
        if dst_mac == &Gateway::MAC {
            self.gateway.process_outbound(pck);
            return;
        }

        // Next we lookup if this destination address is known either
        // on this switch node or another one
        let state = self.data_plane.read().unwrap();
        if let Some(dst) = state.ports.get(&dst_mac) {
            match dst {
                Destination::Local(port) => {
                    let tx = port.tx.clone();

                    self.snoop(state, &pck);
                    let _ = tx.blocking_send(pck);
                    return;
                },
                Destination::PeerSwitch(peer) => {
                    if allow_forward {
                        let pck = self.encrypt_packet(&pck[..]);
                        self.udp.send(&pck[..], peer.clone());
                    }
                }
            }
        }

        // Otherwise we broadcast it to all the other nodes as it
        // could be that we just dont know about it yet or it could
        // be that its multicast/broadcast traffic.
        else if allow_forward {
            self.__broadcast(&state, src, &pck);
        }

        // Snoop the packet
        self.snoop(state, &pck);
    }

    pub fn snoop(&self, state: RwLockReadGuard<DataPlane>, pck: &[u8]) {
        if let Ok(frame_mac) = EthernetFrame::new_checked(&pck[..]) {
            let mac = frame_mac.src_addr();
            if mac.is_unicast() {
                match frame_mac.ethertype() {
                    EthernetProtocol::Ipv4 => {
                        if let Ok(frame_ip) = Ipv4Packet::new_checked(frame_mac.payload()) {
                            let ip = frame_ip.src_addr();

                            let update_mac4 = state.mac4.contains_key(&mac);
                            let update_ip4 = state.ip4.contains_key(&ip);
                            drop(state);

                            if update_mac4 || update_ip4 {
                                let mut state = self.data_plane.write().unwrap();
                                state.mac4.insert(mac, ip, Self::MAC_SNOOP_TTL);
                                state.ip4.insert(ip, mac, Self::MAC_SNOOP_TTL);
                            }
                            return;
                        }
                    },
                    EthernetProtocol::Ipv6 => {
                        if let Ok(frame_ip) = Ipv6Packet::new_checked(frame_mac.payload()) {
                            let ip = frame_ip.src_addr();

                            let update_mac6 = state.mac6.contains_key(&mac);
                            let update_ip6 = state.ip6.contains_key(&ip);
                            drop(state);

                            if update_mac6 || update_ip6 {
                                let mut state = self.data_plane.write().unwrap();
                                state.mac6.insert(mac, ip, Self::MAC_SNOOP_TTL);
                                state.ip6.insert(ip, mac, Self::MAC_SNOOP_TTL);
                            }
                            return;
                        }
                    },
                    EthernetProtocol::Arp => {
                        if let Ok(frame_arp) = ArpPacket::new_checked(frame_mac.payload()) {
                            if frame_arp.hardware_type() == ArpHardware::Ethernet &&
                               frame_arp.protocol_type() == EthernetProtocol::Ipv4
                            {
                                let mac = EthernetAddress::from_bytes(frame_arp.source_hardware_addr());
                                let ip = Ipv4Address::from_bytes(frame_arp.source_protocol_addr());

                                drop(state);
                                let mut state = self.data_plane.write().unwrap();
                                state.mac4.insert(mac, ip, Self::MAC_SNOOP_TTL);
                                state.ip4.insert(ip, mac, Self::MAC_SNOOP_TTL);
                                return;
                            }
                        }
                    },
                    _ => { }
                }
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
        let state = self.data_plane.read().unwrap();
        state.ip4.get(ip).map(|mac| mac.clone())
    }

    pub fn lookup_ipv6(&self, ip: &Ipv6Address) -> Option<EthernetAddress>
    {
        let state = self.data_plane.read().unwrap();
        state.ip6.get(ip).map(|mac| mac.clone())
    }

    pub fn encrypt_packet(&self, pck: &[u8]) -> Vec<u8> {
        let prefix = self.id.to_be_bytes();
        let hash = AteHash::from_bytes(&pck[..]);
        let capacity = prefix.len() + pck.len() + hash.len();
        let mut pck = self.encrypt.encrypt_with_hash_iv_with_capacity_and_prefix(&hash, &pck[..], capacity, &prefix);
        pck.extend_from_slice(hash.as_bytes());
        pck
    }

    pub fn decrypt_packet(&self, data: &[u8], hash: AteHash) -> Option<Vec<u8>> {
        let pck = self.encrypt.decrypt_with_hash_iv(&hash, data);
        let test = AteHash::from_bytes(&pck[..]);
        if test == hash {
            Some(pck)
        } else {
            None
        }
    }

    pub fn process_peer_packet(&self, pck: &[u8], hash: AteHash) {
        if let Some(pck) = self.decrypt_packet(pck, hash) {
            // This should use unicast for destination MAC's that are unicast - other
            // MAC addresses such as multicast and broadcast should use broadcast
            if let Ok(frame) = EthernetFrame::new_checked(&pck[..]) {
                let src = frame.src_addr();
                let dst = frame.dst_addr();
                let _ = self.unicast(&src, &dst, pck, false);
            }
        }
    }

    pub async fn remove_node(&self, node_key: &PrimaryKey)
    {
        if node_key == &self.me_node_key {
            return;
        }

        let mut state = self.data_plane.write().unwrap();
        if let Some(node_addr) = state.peers.remove(node_key) {
            state.ports.retain(|_, v| {
                match v {
                    Destination::PeerSwitch(s) => s == &node_addr,
                    _ => true
                }
            });
        }
    }

    pub async fn update_node(&self, node_key: &PrimaryKey, node: &MeshNode)
    {
        if node_key == &self.me_node_key {
            return;
        }

        let mut state = self.data_plane.write().unwrap();
        state.ports.retain(|_, v| {
            match v {
                Destination::PeerSwitch(s) => s == &node.node_addr,
                _ => true
            }
        });
        state.peers.insert(node_key.clone(), node.node_addr);
        for mac in node.switch_ports.iter() {
            let mac = EthernetAddress::from_bytes(mac.as_bytes());
            state.ports.insert(mac, Destination::PeerSwitch(node.node_addr));
        }
    }

    pub async fn run(&self, mut bus: Bus<MeshNode>, mut mac_drop: mpsc::Receiver<HardwareAddress>)
    {
        debug!("control thread initializing");

        // We first do a full update in this background thread
        // to prevent race conditions missing the updates
        {
            let state = self.control_plane.read().await;
            for node in state.inst.mesh_nodes.iter().await.unwrap() {
                self.update_node(node.key(), node.deref()).await;
            }
        }

        debug!("control thread running");

        loop {
            tokio::select! {
                evt = bus.recv() => {
                    if let Ok(evt) = evt {
                        match evt {
                            BusEvent::Updated(node) => {
                                self.update_node(node.key(), node.deref()).await;
                            },
                            BusEvent::Deleted(key) => {
                                self.remove_node(&key).await;
                            },
                        }
                    } else {
                        break;
                    }
                },
                mac = mac_drop.recv() => {
                    if let Some(mac) = mac {
                        let mut state = self.control_plane.write().await;
                        let dio = state.me_node.dio_mut();
                        {
                            let mut me_node = state.me_node.as_mut();
                            me_node.switch_ports.remove(&mac);
                        }
                        let _ = dio.commit().await;
                    } else {
                        break;
                    }
                }
            }
        }

        debug!("control thread closing");

        // Clear the data plane as we are going offline
        {
            let mut state = self.data_plane.write().unwrap();
            state.peers.clear();
            state.ports.clear();
        }

        // Need to remove the node from the switch in the control plane
        let state = self.control_plane.write().await;
        let dio = state.me_node.dio_mut();
        if dio.delete(state.me_node.key()).await.is_ok() {
            let _ = dio.commit().await;
        }

        debug!("control thread exited");
    }
}