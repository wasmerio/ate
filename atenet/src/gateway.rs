use std::sync::Arc;
use std::sync::MutexGuard;
use std::sync::RwLock;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::EthernetFrame;
use smoltcp::wire::Ipv4Address;
use smoltcp::wire::Ipv4Packet;
use smoltcp::wire::Ipv6Packet;
use smoltcp::wire::EthernetProtocol;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpAddress;
use smoltcp::wire::ArpPacket;
use smoltcp::wire::ArpHardware;
use tokera::model::InstancePeering;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use async_recursion::async_recursion;

use super::switch::Switch;
use super::switch::DataPlane;
use super::factory::SwitchFactory;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Route
{
    cidrs: Vec<IpCidr>,
    switch: Arc<Switch>,
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
    pub(crate) id: u128,
    pub(crate) ips: Vec<IpAddress>,
    pub(crate) state: RwLock<GatewayState>,
    pub(crate) factory: Arc<SwitchFactory>,
}

impl Gateway
{
    pub const MAC: EthernetAddress = EthernetAddress([6u8, 0u8, 0u8, 0u8, 0u8, 1u8]);
    
    pub fn new(id: u128, ips: Vec<IpAddress>, factory: &Arc<SwitchFactory>) -> Gateway {
        Gateway {
            id,
            ips: ips.into_iter().map(|a| a.into()).collect(),
            state: Default::default(),
            factory: factory.clone(),
        }
    }

    #[async_recursion]
    pub async fn update(&self, peerings: Vec<InstancePeering>)
    {
        // Loop through all the peerings with other switches
        let mut routes = Vec::new();
        for peering in peerings
        {
            // Get the switch this is referring to  
            match self.factory.get_or_create_switch(peering.chain.clone()).await {
                Ok((peer_switch, _)) =>
                {
                    // Check to make sure the caller has rights to this switch
                    if peer_switch.has_access(peering.access_token.as_str()) == false {
                        warn!("access to peered switch denied - {}", peering.chain);
                        return;
                    }
                    
                    // Add the route
                    routes.push(Route {
                        cidrs: peer_switch.cidrs(),
                        switch: peer_switch,
                    });
                }
                Err(err) => {
                    warn!("failed to load peered switch - {}", err);
                }
            }
        }

        // Set the routes in the data plane
        let mut state = self.state.write().unwrap();
        state.routes = routes;
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
                    for cidr in route.cidrs.iter() {
                        if cidr.contains_addr(&dst_ip) {
                            if let Some(dst_mac) = route.switch.lookup_ip(&dst_ip) {
                                frame_mac.set_src_addr(Self::MAC);
                                frame_mac.set_dst_addr(dst_mac);
                                drop(frame_mac);
                                
                                route.switch.unicast(&Self::MAC, &dst_mac, pck, true);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn process_arp_reply(&self, pck: &[u8], switch: &Switch, state: &mut MutexGuard<DataPlane>)
    {
        if let Ok(frame_mac) = EthernetFrame::new_checked(pck) {
            if frame_mac.dst_addr() == EthernetAddress::BROADCAST &&
               frame_mac.ethertype() == EthernetProtocol::Arp
            {
                let src_mac = frame_mac.src_addr();

                if let Ok(frame_arp) = ArpPacket::new_checked(frame_mac.payload())
                {
                    if frame_arp.hardware_type() == ArpHardware::Ethernet &&
                       frame_arp.protocol_type() == EthernetProtocol::Ipv4
                    {
                        let ip = IpAddress::Ipv4(Ipv4Address::from_bytes(frame_arp.target_protocol_addr()));
                        for gateway_ip in self.ips.iter().filter(|i| *i == &ip) {
                            let mut pck = pck.to_vec();

                            let mut frame_mac = EthernetFrame::new_unchecked(&mut pck[..]);
                            frame_mac.set_src_addr(Self::MAC);
                            frame_mac.set_dst_addr(src_mac);
                            
                            let mut frame_arp = ArpPacket::new_unchecked(frame_mac.payload_mut());
                            frame_arp.set_source_hardware_addr(src_mac.as_bytes());
                            frame_arp.set_source_protocol_addr(gateway_ip.as_bytes());

                            drop(frame_arp);
                            drop(frame_mac);
                            switch.__unicast(state, &Self::MAC, &src_mac, pck, true);
                        }
                    }
                }
            }
        }
    }
}