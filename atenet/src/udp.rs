use std::sync::Arc;
use std::net::IpAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use std::net::SocketAddr;
use std::sync::RwLock;
use std::collections::HashMap;
use std::sync::Weak;
use std::io::Read;
use bytes::Bytes;
use ate::crypto::AteHash;
use byteorder::{BigEndian, ReadBytesExt};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::switch::Switch;
use super::common::get_local_ip;

struct UdpSend
{
    buf: Bytes,
    addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct UdpPeerHandle
{
    tx: mpsc::Sender<UdpSend>,
    addr: SocketAddr,
}

impl UdpPeerHandle
{
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn local_ip(&self) -> IpAddr {
        self.addr.ip()
    }

    pub fn local_port(&self) -> u16 {
        self.addr.port()
    }

    pub fn send(&self, buf: Bytes, addr: IpAddr) {
        let addr = SocketAddr::new(addr, self.local_port());
        if let Err(mpsc::error::TrySendError::Full(send)) = self.tx.try_send(UdpSend {
            buf,
            addr,
        }) {
            let tx = self.tx.clone();
            tokio::task::spawn(async move {
                let _ = tx.send(send).await;
            });
        }
    }
}

#[derive(Debug)]
pub struct UdpPeer
{
    addr: SocketAddr,
    send: mpsc::Receiver<UdpSend>,
    socket: Arc<UdpSocket>,
    switches: Arc<RwLock<HashMap<u128, Weak<Switch>>>>,
}

impl UdpPeer
{
    pub async fn new(ip: IpAddr, port: u16, switches: Arc<RwLock<HashMap<u128, Weak<Switch>>>>) -> UdpPeerHandle
    {
        let addr = if ip.is_loopback() {
            ip
        } else {
            get_local_ip()
        };
        let addr = SocketAddr::new(addr, port); 
        let socket = UdpSocket::bind(addr).await.unwrap();

        let (send_tx, send_rx) = mpsc::channel(super::common::MAX_MPSC);
        let udp = UdpPeer {
            addr,
            send: send_rx,
            socket: Arc::new(socket),
            switches,
        };

        tokio::task::spawn(async move {
            udp.run().await;
        });
        
        UdpPeerHandle {
            tx: send_tx,
            addr,
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

    pub async fn run(mut self) {
        let mut buf = [0u8; 10000];
        debug!("UDP server started - {}", self.addr);
        loop {
            tokio::select! {
                size = self.socket.recv_from(&mut buf[..]) => {
                    if let Ok((size, peer)) = size {
                        let mut pck = &buf[..size];
                        
                        if let Ok(id) = pck.read_u128::<BigEndian>() {

                            let mut hash = [0u8; 16];
                            if let Ok(()) = pck.read_exact(&mut hash) {
                                if pck.len() > 0 {
                                    self.process(id, &pck[..], hash.into(), peer);
                                } else {
                                    debug!("packet dropped - no data");
                                }
                            } else {
                                debug!("packet dropped - read hash failed");
                            }                            
                        } else {
                            debug!("packet dropped - read id failed");
                        }
                    } else {
                        break;
                    }
                },
                send = self.send.recv() => {
                    if let Some(send) = send {
                        let _ = self.socket.send_to(&send.buf[..], send.addr).await;
                    } else  {
                        break;
                    }
                }
            }
        }
        debug!("UDP server closed - {}", self.addr);
    }

    pub fn process(&self, id: u128, data: &[u8], hash: AteHash, peer: SocketAddr) {
        let switches = self.switches.read().unwrap();
        if let Some(switch) = switches.get(&id) {
            if let Some(switch) = switch.upgrade() {
                switch.process_peer_packet(data, hash, peer.ip());
            } else {
                debug!("packet dropped - weak switch id={}", id);    
            }
        } else {
            debug!("packet dropped - orphaned switch id={}", id);
        }
    }
}