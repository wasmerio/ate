#![allow(unused_variables)]
use std::net::SocketAddr;
use std::net::SocketAddrV4;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

mod common;

#[test]
fn udp_simple_static() {
    run_udp(false, false);
}

#[test]
fn udp_mesh_static() {
    run_udp(true, false);
}

#[test]
fn udp_simple_dhcp() {
    run_udp(false, true);
}

#[test]
fn udp_mesh_dhcp() {
    run_udp(true, true);
}

fn run_udp(cross_switch: bool, use_dhcp: bool) {
    common::run(async move {
        let _servers = common::setup().await;

        let (c1, c2) = common::clients(cross_switch, use_dhcp).await;
        let s1_addr = SocketAddr::V4(SocketAddrV4::new(c1.addr_ipv4().await.unwrap(), 3000));
        let s2_addr = SocketAddr::V4(SocketAddrV4::new(c2.addr_ipv4().await.unwrap(), 3000));
        
        let mut s1 = c1.bind_udp(s1_addr).await.unwrap();
        let mut s2 = c2.bind_udp(s2_addr).await.unwrap();
        
        tokio::task::spawn(async move {
            loop {
                let (test, peer) = s1.recv_from().await.unwrap();
                assert_eq!(test, vec![1,2,3]);
                s1.send_to(vec![4,5,6], peer).await.unwrap();    
            }
        });

        loop {
            s2.send_to(vec![1,2,3], s1_addr).await.unwrap();
            if let Ok(Some((test, addr))) = s2.try_recv_from() {
                assert_eq!(test, vec![4,5,6]);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
}