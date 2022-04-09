#![allow(unreachable_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::ops::*;
use ate_files::prelude::FileAccessor;
use tokio::sync::mpsc;
use smoltcp::wire::EthernetAddress;
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
    pub(crate) peers: Vec<IpAddr>,
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
    #[allow(dead_code)]
    pub(crate) accessor: Arc<FileAccessor>,
    pub(crate) data_plane: StdRwLock<DataPlane>,
    pub(crate) control_plane: RwLock<ControlPlane>,
    pub(crate) mac_drop: mpsc::Sender<HardwareAddress>,
}

impl Switch
{
    pub async fn new(accessor: Arc<FileAccessor>, addr: IpAddr) -> Result<Arc<Switch>, AteError> {
        let (inst, bus, me_node) = {
            let chain_dio = accessor.dio.clone().as_mut().await;
            
            let mut inst = chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;

            let me_node = {
                let mut inst = inst.as_mut();
                match inst
                    .mesh_nodes
                    .iter_mut()
                    .await?
                    .filter(|m| m.node_addr == addr)
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
                            node_addr: addr,
                            switch_ports: Default::default(),
                        })?
                    }
                }
            };            
            chain_dio.commit().await?;

            let bus = inst.mesh_nodes.bus().await?;
            (inst, bus, me_node)
        };

        let (mac_drop_tx, mac_drop_rx) = mpsc::channel(100);
        let switch = Arc::new(Switch {
            accessor,
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
        for (mac, dst) in state.ports.iter() {
            if let Destination::Local(port) = dst {
                if src != mac {
                    let _ = port.tx.blocking_send(pck.clone());
                }
            }
        }
        for _peer in state.peers.iter() {
            todo!();
        }
    }

    pub fn unicast(&self, src: &EthernetAddress, dst_mac: &EthernetAddress, pck: Vec<u8>, allow_forward: bool) {

        // If the destination is the default gateway then we need to take a
        // look at this packet and route it somewhere (either the internet or
        // anothe switch network)
        //todo!();

        let state = self.data_plane.read().unwrap();
        if let Some(dst) = state.ports.get(&dst_mac) {
            match dst {
                Destination::Local(port) => {
                    let _ = port.tx.blocking_send(pck);    
                },
                Destination::PeerSwitch(_peer) => {
                    if allow_forward {
                        todo!();
                    }
                }
            }
        } else if allow_forward {
            self.broadcast(src, pck);
        }
    }

    pub async fn update_node(&self, _node: &MeshNode) {
        // Update all the routing tables using the node data
        todo!();
    }

    pub async fn run(&self, mut bus: Bus<MeshNode>, mut mac_drop: mpsc::Receiver<HardwareAddress>)
    {
        debug!("control thread initializing");

        // We first do a full update in this background thread
        // to prevent race conditions missing the updates
        {
            let state = self.control_plane.read().await;
            for node in state.inst.mesh_nodes.iter().await.unwrap() {
                self.update_node(node.deref()).await;
            }
        }

        debug!("control thread running");

        loop {
            tokio::select! {
                evt = bus.recv() => {
                    if let Ok(evt) = evt {
                        match evt {
                            BusEvent::Updated(node) => {
                                self.update_node(node.deref()).await;
                            },
                            BusEvent::Deleted(_key) => {
                                todo!();
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