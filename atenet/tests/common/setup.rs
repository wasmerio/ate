use std::sync::Arc;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::future::Future;
use ate::prelude::*;
use atenet::opt::OptsNetworkServer;
use tokio::runtime::Builder;
use tokera::mio::Port;


fn create_solo(ip: IpAddr, node_id: u32) -> OptsNetworkServer
{
    OptsNetworkServer {
        listen: ip,
        http_port: Some(8080),
        tls_port: Some(4443),
        udp_port: Some(2222),
        token_path: "~/ate/token".to_string(),
        db_url: url::Url::parse("ws://tokera.sh/db").unwrap(),
        auth_url: url::Url::parse("ws://tokera.sh/auth").unwrap(),
        inst_url: url::Url::parse("ws://tokera.sh/inst").unwrap(),
        node_id: Some(node_id),
        ttl: 300,
    }   
}

async fn create_node(ip: IpAddr, node_id: u32) -> Arc<ateweb::server::Server> {
    let conf = AteConfig::default();
    let solo = create_solo(ip, node_id);
    
    let (server, _exit) = atenet::common::setup_server(
        solo,
        conf,
        None,
        None
    ).await.unwrap();

    server
}

pub fn run<F: Future>(future: F) -> F::Output {
    let runtime = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());
    runtime.clone().block_on(future)
}

pub async fn setup() -> Vec<Arc<ateweb::server::Server>> {
    ate::log_init(4, true);

    let s1 = create_node(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 1).await;
    let s2 = create_node(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)), 2).await;

    vec![s1, s2]
}

pub async fn client1(ip: IpAddr, chain: &ChainKey, access_token: &str) -> Port
{
    let node = url::Url::parse("ws://127.0.0.2:8080/net").unwrap();
    client(node, ip, chain.clone(), access_token.to_string()).await
}

pub async fn client2(ip: IpAddr, chain: &ChainKey, access_token: &str) -> Port
{
    let node = url::Url::parse("ws://127.0.0.3:8080/net").unwrap();
    client(node, ip, chain.clone(), access_token.to_string()).await
}

pub async fn client(node: url::Url, ip: IpAddr, chain: ChainKey, access_token: String) -> Port
{
    Port::connect(node, chain, access_token).await.unwrap()
}