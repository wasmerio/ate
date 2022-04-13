#![allow(unused_variables)]
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use ate::chain::ChainKey;

mod common;

#[test]
fn ping() {
    common::run(async move {
        let _servers = common::setup().await;

        let chain = ChainKey::from("johnathan.sharratt@gmail.com/17ccd5c3d5eb84c56c717ef7fbd7c4ae_edge");
        let access_token = "409454760a1d4f0fbad6cb1edc15e6ac";
        
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