#![allow(unreachable_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::ops::*;
use std::sync::RwLockReadGuard;
use ate_files::prelude::FileAccessor;
use tokio::sync::mpsc;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::EthernetFrame;
use derivative::*;
use tokio::sync::RwLock;
use std::sync::RwLock as StdRwLock;
use ate::prelude::*;
use tokera::model::MeshNode;
use tokera::model::HardwareAddress;
use tokera::model::ServiceInstance;
use tokera::model::INSTANCE_ROOT_ID;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::port::*;
use super::common::*;
use super::udp::*;

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
}

impl Switch
{
    pub async fn new(accessor: Arc<FileAccessor>, udp: UdpPeer) -> Result<Arc<Switch>, AteError> {
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
                }
            ),
            control_plane: RwLock::new(
                ControlPlane {
                    inst,
                    me_node,
                }
            ),
            mac_drop: mac_drop_tx,
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
        self.__broadcast(&state, src, pck);
    }
    
    fn __broadcast(&self, state: &RwLockReadGuard<DataPlane>, src: &EthernetAddress, pck: Vec<u8>) {
        for (mac, dst) in state.ports.iter() {
            if let Destination::Local(port) = dst {
                if src != mac {
                    let _ = port.tx.blocking_send(pck.clone());
                }
            }
        }

        // Encrypt and sign the packet before we send it
        let pck = self.encrypt_packet(&pck[..]);
        for peer in state.peers.values() {
            self.udp.send(&pck[..], peer.clone());
        }
    }

    pub fn unicast(&self, src: &EthernetAddress, dst_mac: &EthernetAddress, pck: Vec<u8>, allow_forward: bool) {

        let state = self.data_plane.read().unwrap();
        if let Some(dst) = state.ports.get(&dst_mac) {
            match dst {
                Destination::Local(port) => {
                    let _ = port.tx.blocking_send(pck);    
                },
                Destination::PeerSwitch(peer) => {
                    if allow_forward {
                        let pck = self.encrypt_packet(&pck[..]);
                        self.udp.send(&pck[..], peer.clone());
                    }
                }
            }
        } else if allow_forward {
            self.__broadcast(&state, src, pck);
        }
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