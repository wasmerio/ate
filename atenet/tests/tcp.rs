#![allow(unused_variables)]
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use ate::chain::ChainKey;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

mod common;

#[test]
fn ping() {
    common::run(async move {
        let _servers = common::setup().await;

        let chain = ChainKey::from("tokera.com/4932a508739386ec3c4d76d269fc30eb_edge");
        let access_token = "4d6d309c1e9c58d3b3493c0fd00554f1";
        
        let c1_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 2), 3000));
        let c1 = common::client1(c1_addr.ip().clone(), &chain, access_token).await;
        
        let c2_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 3), 3000));
        let c2 = common::client2(c2_addr.ip().clone(), &chain, access_token).await;
        
        let mut s1 = c1.listen_tcp(c1_addr).await.unwrap();
        let s2 = c2.connect_tcp(c2_addr, c1_addr).await.unwrap();
        s1.accept().await.unwrap();
        s2.send(vec![1,2,3]).await.unwrap();
        let test = s1.recv().await.unwrap();
        
        assert_eq!(test, vec![1,2,3])
    })
}