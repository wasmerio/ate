use std::collections::HashMap;
use std::sync::Arc;
use ate_files::prelude::FileAccessor;
use tokio::sync::mpsc;
use smoltcp::wire::EthernetAddress;
use derivative::*;
use std::sync::RwLock;

use super::port::*;
use super::common::*;

pub struct SwitchPort {
    tx: mpsc::Sender<Vec<u8>>,
    #[allow(dead_code)]
    mac: EthernetAddress,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Switch
{
    #[allow(dead_code)]
    pub(crate) accessor: Arc<FileAccessor>,
    #[derivative(Debug = "ignore")]
    pub(crate) ports: RwLock<HashMap<EthernetAddress, SwitchPort>>,
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
            let mut ports = self.ports.write().unwrap();
            ports.insert(mac, switch_port);
        }

        Port::new(self, mac, rx)
    }
    
    pub fn broadcast(&self, src: &EthernetAddress, pck: Vec<u8>) {
        let ports = self.ports.read().unwrap();
        for (dst, port) in ports.iter() {
            if src != dst {
                let _ = port.tx.blocking_send(pck.clone());
            }
        }
    }

    pub fn unicast(&self, dst: &EthernetAddress, pck: Vec<u8>) {
        let ports = self.ports.read().unwrap();
        for (dst2, port) in ports.iter() {
            if dst != dst2 {
                let _ = port.tx.blocking_send(pck);
                break;
            }
        }
    }
}