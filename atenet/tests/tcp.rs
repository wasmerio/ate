#![allow(unused_variables)]
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use ate::chain::ChainKey;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

mod common;

#[test]
fn test_tcp_mesh() {
    common::run(async move {
        let _servers = common::setup().await;

        let chain = ChainKey::from("tokera.com/e7cc8d8528b79d6975bcf438f7308f78_edge");
        let access_token = "27801ccc9ada31487c5fce7dc2d41078";
        
        let c1_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 180, 41, 2), 3000));
        let c1 = common::client1(c1_addr.ip().clone(), &chain, access_token).await;
        
        let c2_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 180, 41, 3), 3000));
        let c2 = common::client2(c2_addr.ip().clone(), &chain, access_token).await;
        
        let mut s1 = c1.listen_tcp(c1_addr).await.unwrap();
        tokio::task::spawn(async move {
            s1.accept().await.unwrap();
            let test = s1.recv().await.unwrap();
            assert_eq!(test, vec![1,2,3]);
            s1.send(vec![4,5,6]).await.unwrap();
        });
        
        let mut s2 = c2.connect_tcp(c2_addr, c1_addr).await.unwrap();
        s2.send(vec![1,2,3]).await.unwrap();
        let test = s2.recv().await.unwrap();
        assert_eq!(test, vec![4,5,6]);
    })
}