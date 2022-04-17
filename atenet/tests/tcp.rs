#![allow(unused_variables)]
use std::net::SocketAddr;
use std::net::SocketAddrV4;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use serial_test::serial;

mod common;

#[test]
#[serial]
fn tcp_simple_static() {
    run_tcp(false, false);
}

#[test]
#[serial]
fn tcp_mesh_static() {
    run_tcp(true, false);
}

#[test]
#[serial]
fn tcp_simple_dhcp() {
    run_tcp(false, true);
}

#[test]
#[serial]
fn tcp_mesh_dhcp() {
    run_tcp(true, true);
}

fn run_tcp(cross_switch: bool, use_dhcp: bool) {
    common::run(async move {
        let _servers = common::setup().await;

        let (c1, c2) = common::clients(cross_switch, use_dhcp).await;
        let s1_addr = SocketAddr::V4(SocketAddrV4::new(c1.addr_ipv4().await.unwrap(), 3000));
        let s2_addr = SocketAddr::V4(SocketAddrV4::new(c2.addr_ipv4().await.unwrap(), 3000));
        
        let mut s1 = c1.listen_tcp(s1_addr).await.unwrap();
        tokio::task::spawn(async move {
            s1.accept().await.unwrap();
            let test = s1.recv().await.unwrap();
            assert_eq!(test, vec![1,2,3]);
            s1.send(vec![4,5,6]).await.unwrap();
        });
        
        let mut s2 = c2.connect_tcp(s2_addr, s1_addr).await.unwrap();
        s2.send(vec![1,2,3]).await.unwrap();
        let test = s2.recv().await.unwrap();
        assert_eq!(test, vec![4,5,6]);
    })
}