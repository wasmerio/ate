use std::collections::HashMap;
use std::sync::Arc;
use ate_files::prelude::FileAccessor;
use tokio::sync::mpsc;
use smoltcp::wire::EthernetAddress;
use derivative::*;
use std::sync::RwLock;
use ate::prelude::*;
use tokera::model::MeshNode;

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
#[derivative(Debug, Default)]
pub struct SwitchState {
    pub(crate) ports: HashMap<EthernetAddress, Destination>,
    pub(crate) peers: Vec<IpAddr>,
}

#[derive(Debug)]
pub struct Switch
{
    #[allow(dead_code)]
    pub(crate) accessor: Arc<FileAccessor>,
    pub(crate) state: RwLock<SwitchState>,
}

impl Switch
{
    pub fn new_port(self: &Arc<Switch>) -> Port {
        let mac = EthernetAddress::default();
        let (tx, rx) = mpsc::channel(MAX_MPSC);
        let switch_port = SwitchPort {
            tx,
            mac,
        };

        {
            let mut state = self.state.write().unwrap();
            state.ports.insert(mac, Destination::Local(switch_port));
        }

        let ret = Port::new(self, mac, rx);

        // Update the chain with the new record
        todo;

        ret
    }
    
    pub fn broadcast(&self, src: &EthernetAddress, pck: Vec<u8>) {
        let state = self.state.read().unwrap();
        for (mac, dst) in state.ports.iter() {
            if let Destination::Local(port) = dst {
                if src != mac {
                    let _ = port.tx.blocking_send(pck.clone());
                }
            }
        }
        for peer in state.peers.iter() {
            todo;
        }
    }

    pub fn unicast(&self, src: &EthernetAddress, dst_mac: &EthernetAddress, pck: Vec<u8>, allow_forward: bool) {

        // If the destination is the default gateway then we need to take a
        // look at this packet and route it somewhere (either the internet or
        // anothe switch network)
        //
        // This is also where outbound firewalls should be checked before the packets
        // are actually routed anywhere.
        todo;

        let state = self.state.read().unwrap();
        if let Some(dst) = state.ports.get(&dst_mac) {
            match dst {
                Destination::Local(port) => {
                    let _ = port.tx.blocking_send(pck);    
                },
                Destination::PeerSwitch(peer) => {
                    if allow_forward {
                        todo;
                    }
                }
            }
        } else if allow_forward {
            self.broadcast(src, pck);
        }
    }

    pub async fn run(&self, mut bus: Bus<MeshNode>) {
        while let Ok(evt) = bus.recv().await {
            match evt {
                BusEvent::Updated(node) => {
                    todo;
                },
                BusEvent::Deleted(key) => {
                    todo;
                },
                _ => { }
            }
        }
    }
}