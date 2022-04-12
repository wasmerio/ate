#![allow(unused_variables)]
use std::net::{IpAddr, Ipv4Addr};
use ate::chain::ChainKey;

mod common;

#[test]
fn ping() {
    common::run(async move {
        let _servers = common::setup().await;

        let chain = ChainKey::from("johnathan.sharratt@gmail.com/17ccd5c3d5eb84c56c717ef7fbd7c4ae_edge");
        let access_token = "409454760a1d4f0fbad6cb1edc15e6ac";
        
        let c1 = common::client1(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), &chain, access_token).await;
        let c2 = common::client2(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), &chain, access_token).await;
    })
}