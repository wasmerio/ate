use std::sync::Arc;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use ate::prelude::*;
use ate::comms::StreamRouter;
use ate::mesh::MeshHashTable;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpAddress;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};

use crate::opt::*;
use crate::server::Server;

pub const MAX_MPSC: usize = std::usize::MAX >> 3;
pub const MAX_BROADCAST: usize = std::usize::MAX >> 2;

pub fn get_local_ip() -> IpAddr
{
    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();
    for local_ip in local_ips.iter() {
        if local_ip.is_ipv4() {
            if is_ip_global(local_ip) {
                return local_ip.clone();
            }
        }
    }
    for local_ip in local_ips.iter() {
        if is_ip_global(local_ip) {
            return local_ip.clone();
        }
    }
    panic!("unable to determine the local global IP address");
}

pub const fn is_ip_global(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_ip4_global(ip),
        IpAddr::V6(ip) => is_ip6_global(ip),
    }
}

pub const fn is_ip4_shared(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] == 100 && (ip.octets()[1] & 0b1100_0000 == 0b0100_0000)
}

pub const fn is_ip4_reserved(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] & 240 == 240 && !ip.is_broadcast()
}

pub const fn is_ip4_benchmarking(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] == 198 && (ip.octets()[1] & 0xfe) == 18
}

pub const fn is_ip4_global(ip: &Ipv4Addr) -> bool {
    // check if this address is 192.0.0.9 or 192.0.0.10. These addresses are the only two
    // globally routable addresses in the 192.0.0.0/24 range.
    if u32::from_be_bytes(ip.octets()) == 0xc0000009
        || u32::from_be_bytes(ip.octets()) == 0xc000000a
    {
        return true;
    }
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_broadcast()
        && !ip.is_documentation()
        && !is_ip4_shared(ip)
        // addresses reserved for future protocols (`192.0.0.0/24`)
        && !(ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
        && !is_ip4_reserved(ip)
        && !is_ip4_benchmarking(ip)
        // Make sure the address is not in 0.0.0.0/8
        && ip.octets()[0] != 0
}

pub const fn is_ip6_global(ip: &Ipv6Addr) -> bool {
    match ip.segments()[0] & 0x000f {
        1 => false,
        2 => false,
        3 => false,
        4 => false,
        5 => false,
        8 => false,
        14 => true,
        _ => is_ip6_unicast_global(ip),
    }
}

pub const fn is_ip6_unicast_global(ip: &Ipv6Addr) -> bool {
    is_ip6_unicast(ip)
        && !ip.is_loopback()
        && !is_ip6_unicast_link_local(ip)
        && !is_ip6_unique_local(ip)
        && !ip.is_unspecified()
        && !is_ip6_documentation(ip)
}

pub const fn is_ip6_unicast(ip: &Ipv6Addr) -> bool {
    !ip.is_multicast()
}

pub const fn is_ip6_unicast_link_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

pub const fn is_ip6_unique_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

pub const fn is_ip6_documentation(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] == 0x2001) && (ip.segments()[1] == 0xdb8)
}

#[allow(dead_code)]
pub async fn setup_web(solo: &OptsNetworkServer, cfg_ate: ConfAte, callback: Option<StreamRouter>) -> Result<(Arc<ateweb::server::Server>, watch::Receiver<bool>), AteError>
{
    let (hard_exit_tx, hard_exit_rx) = tokio::sync::watch::channel(false);
    let server = setup_web_ext(solo, cfg_ate, callback, hard_exit_tx).await?;
    Ok((server, hard_exit_rx))
}

pub async fn setup_web_ext(solo: &OptsNetworkServer, cfg_ate: ConfAte, callback: Option<StreamRouter>, hard_exit_tx: watch::Sender<bool>) -> Result<Arc<ateweb::server::Server>, AteError>
{
    let mut builder = ateweb::builder::ServerBuilder::new(solo.db_url.clone(), solo.auth_url.clone())
        .add_listener(solo.listen, solo.http_port.unwrap_or(80u16), false)
        .add_listener(solo.listen, solo.tls_port.unwrap_or(443u16), true)
        .with_conf(&cfg_ate);

    if let Some(callback) = callback {
        builder = builder
            .with_callback(callback);
    }

    let server = builder
        .build()
        .await?;

    // Run the web server
    {
        let server = Arc::clone(&server);
        TaskEngine::spawn(async move {
            let ret = server.run().await;
            if let Err(err) = ret {
                error!("web server fatal error - {}", err);
            }
            let _ = hard_exit_tx.send(true);
        });
    }

    // Done
    Ok(server)
}

pub async fn setup_server
(
    solo: OptsNetworkServer,
    conf: AteConfig,
    wire_encryption: Option<KeySize>,
    listen_certificate: Option<PrivateEncryptKey>
) -> Result<(Arc<ateweb::server::Server>, watch::Receiver<bool>), Box<dyn std::error::Error>>
{
    let protocol = StreamProtocol::parse(&solo.inst_url)?;
    let port = solo.auth_url.port().unwrap_or(protocol.default_port());
    let domain = solo.auth_url.domain().unwrap_or("localhost").to_string();
    let ttl = std::time::Duration::from_secs(solo.ttl);

    let mut cfg_mesh = ConfMesh::skeleton(&conf, domain, port, solo.node_id).await?;
    cfg_mesh.wire_protocol = protocol;
    cfg_mesh.wire_encryption = wire_encryption;
    cfg_mesh.listen_certificate = listen_certificate;

    let table = MeshHashTable::new(&cfg_mesh);
    let server_id = table.compute_node_id(solo.node_id)?;

    let registry = Arc::new(Registry::new(&conf).await);

    let mut instance_authority = solo.inst_url.domain()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "tokera.sh".to_string());
    if instance_authority == "localhost" {
        instance_authority = "tokera.sh".to_string();
    }

    let mut router = ate::comms::StreamRouter::new(
        cfg_mesh.wire_format.clone(),
        cfg_mesh.wire_protocol.clone(),
        cfg_mesh.listen_certificate.clone(),
        server_id,
        cfg_mesh.accept_timeout,
    );

    let instance_server = Server::new(
        solo.db_url.clone(),
        solo.auth_url.clone(),
        instance_authority,
        solo.token_path.clone(),
        registry,
        ttl,
        solo.listen.clone(),
        solo.udp_port.unwrap_or(2000),
    ).await?;
    
    let route = Arc::new(instance_server);
    router.add_socket_route("/net", route.clone()).await;
    
    Ok(
        setup_web(&solo, conf, Some(router)).await?
    )
}

pub fn subnet_to_cidrs(subnet: &tokera::model::InstanceSubnet) -> Vec<IpCidr>
{
    subnet.cidrs.iter()
            .map(|cidr| IpCidr::new(cidr.ip.into(), cidr.prefix))
            .collect()
}

pub fn cidr_to_gw(cidr: &IpCidr) -> IpAddress
{
    match cidr.address() {
        IpAddress::Ipv4(ip) => {
            let ip: Ipv4Addr = ip.into();
            let mut ip: u32 = ip.into();
            ip += 1;
            IpAddr::V4(ip.into()).into()
        },
        IpAddress::Ipv6(ip) => {
            let ip: Ipv6Addr = ip.into();
            let mut ip: u128 = ip.into();
            ip += 1;
            IpAddr::V6(ip.into()).into()
        },
        _ => {
            IpAddress::Unspecified
        }
    }
}