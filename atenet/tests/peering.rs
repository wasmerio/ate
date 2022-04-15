#![allow(unused_variables)]
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use ate::chain::ChainKey;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

mod common;

#[test]
fn test_udp_mesh() {
    common::run(async move {
        let _servers = common::setup().await;

        let chain1 = ChainKey::from("tokera.com/e7cc8d8528b79d6975bcf438f7308f78_edge");
        let access_token1 = "27801ccc9ada31487c5fce7dc2d41078";

        let chain2 = ChainKey::from("tokera.com/c23a55096856cc316fb4da3f7a878192_edge");
        let access_token2 = "6ac4fcf2716b1a22fc298c6d526f031e";
        
        let s1_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 180, 41, 2), 3000));
        let c1 = common::client1(s1_addr.ip().clone(), &chain1, access_token1).await;
        
        let s2_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 164, 156, 2), 3000));
        let c2 = common::client2(s2_addr.ip().clone(), &chain2, access_token2).await;
        
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