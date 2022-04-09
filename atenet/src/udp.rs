use std::sync::Arc;
use std::net::IpAddr;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::RwLock;
use std::collections::HashMap;
use std::sync::Weak;
use ate::crypto::AteHash;
use byteorder::{BigEndian, ReadBytesExt}; 

use super::switch::Switch;
use super::common::get_local_ip;

#[derive(Debug, Clone)]
pub struct UdpPeer
{
    addr: SocketAddr,
    socket: Arc<UdpSocket>,
    switches: Arc<RwLock<HashMap<u128, Weak<Switch>>>>,
}

impl UdpPeer
{
    pub fn new(port: u16, switches: Arc<RwLock<HashMap<u128, Weak<Switch>>>>) -> UdpPeer
    {
        let addr = get_local_ip();
        let addr = SocketAddr::new(addr, port); 
        let socket = UdpSocket::bind(addr).unwrap();

        UdpPeer {
            addr,
            socket: Arc::new(socket),
            switches,
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn local_ip(&self) -> IpAddr {
        self.addr.ip()
    }

    pub fn local_port(&self) -> u16 {
        self.addr.port()
    }

    pub fn send(&self, buf: &[u8], addr: IpAddr) {
        let addr = SocketAddr::new(addr, self.local_port());
        let _ = self.socket.send_to(buf, addr);
    }

    pub fn run(&self) {
        let mut buf = [0u8; 10000];
        loop {
            if let Ok(size) = self.socket.recv(&mut buf[..]) {
                let mut pck = &buf[..size];
                
                if let Ok(id) = pck.read_u128::<BigEndian>() {
                    if pck.len() > AteHash::LEN {
                        let size = pck.len() - AteHash::LEN;
                        
                        let hash = &pck[size..];
                        let hash = AteHash::from_bytes(hash);

                        let data = &pck[..size];

                        self.process(id, data, hash);
                    }
                }
            }
        }
    }

    pub fn process(&self, id: u128, data: &[u8], hash: AteHash) {
        let switches = self.switches.read().unwrap();
        if let Some(switch) = switches.get(&id) {
            if let Some(switch) = switch.upgrade() {
                switch.process_peer_packet(data, hash);
            }
        }
    }
}