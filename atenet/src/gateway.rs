use std::sync::Arc;
use std::sync::Weak;
use std::sync::RwLock;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::EthernetFrame;
use smoltcp::wire::Ipv4Packet;
use smoltcp::wire::Ipv6Packet;
use smoltcp::wire::EthernetProtocol;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpAddress;

use super::switch::Switch;
use super::factory::SwitchFactory;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Route
{
    id: u128,
    cidr: IpCidr,
    switch: Weak<Switch>,
    access_code: String,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct GatewayState
{
    routes: Vec<Route>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Gateway
{
    id: u128,
    state: RwLock<GatewayState>,
    factory: Arc<SwitchFactory>,
}

impl Gateway
{
    pub const MAC: EthernetAddress = EthernetAddress([6u8, 0u8, 0u8, 0u8, 0u8, 1u8]);
    
    pub fn new(id: u128, factory: &Arc<SwitchFactory>) -> Gateway {
        Gateway {
            id,
            state: Default::default(),
            factory: factory.clone(),
        }
    }

    pub fn process_outbound(&self, mut pck: Vec<u8>)
    {
        // Packets going to another switch that we have a routing table entry
        // to need to have their MAC address modified then transmitted
        let state = self.state.read().unwrap();
        if let Ok(mut frame_mac) = EthernetFrame::new_checked(&mut pck[..]) {
            let dst_ip = match frame_mac.ethertype() {
                EthernetProtocol::Ipv4 => {
                    if let Ok(frame_ip) = Ipv4Packet::new_checked(frame_mac.payload_mut()) {
                        let dst_ip = IpAddress::Ipv4(frame_ip.dst_addr());
                        drop(frame_ip);
                        Some(dst_ip)
                    } else {
                        None
                    }
                }
                EthernetProtocol::Ipv6 => {
                    if let Ok(frame_ip) = Ipv6Packet::new_checked(frame_mac.payload_mut()) {
                        let dst_ip = IpAddress::Ipv6(frame_ip.dst_addr());
                        drop(frame_ip);
                        Some(dst_ip)
                    } else {
                        None
                    }
                }
                _ => None
            };

            if let Some(dst_ip) = dst_ip {
                for route in state.routes.iter() {
                    if route.cidr.contains_addr(&dst_ip) {
                        if let Some(switch) = route.switch.upgrade() {
                            if let Some(dst_mac) = switch.lookup_ip(&dst_ip) {
                                frame_mac.set_src_addr(Self::MAC);
                                frame_mac.set_dst_addr(dst_mac);
                                drop(frame_mac);
                                
                                switch.unicast(&Self::MAC, &dst_mac, pck, true);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn process_inbound(&self, _pck: &[u8]) {

    }
}