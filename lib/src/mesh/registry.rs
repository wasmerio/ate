#![allow(unused_imports)]
use log::{warn, debug, error};
use crate::{
    conf::ConfAte,
    error::ChainCreationError
};
use crate::chain::Chain;
use crate::chain::ChainKey;
use crate::mesh::*;
use crate::error::*;
use crate::conf::ConfCluster;
use std::{net::IpAddr, sync::Arc};
use fxhash::FxHashMap;
use tokio::sync::Mutex;
use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use tokio::net::TcpStream as TokioTcpStream;
use tokio::net::UdpSocket as TokioUdpSocket;
use url::Url;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::str::FromStr;
use trust_dns_client::client::{Client, SyncClient, ClientHandle, AsyncClient, MemoizeClientHandle};
use trust_dns_client::error::ClientError;
use trust_dns_client::udp::UdpClientConnection;
use trust_dns_client::udp::UdpClientStream;
use trust_dns_client::tcp::TcpClientConnection;
use trust_dns_client::tcp::TcpClientStream;
use trust_dns_client::tcp::TcpStream;
use trust_dns_client::op::DnsResponse;
use trust_dns_client::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_proto::DnssecDnsHandle;
use trust_dns_proto::iocompat::AsyncIoTokioAsStd;
use trust_dns_client::op::ResponseCode;
use trust_dns_client::rr::dnssec::TrustAnchor;
use trust_dns_proto::{
    error::ProtoError,
    xfer::{DnsHandle, DnsRequest},
};

enum DnsClient
{
    Dns(MemoizeClientHandle<AsyncClient>),
    DnsSec(DnssecDnsHandle<MemoizeClientHandle<AsyncClient>>)
}

impl DnsClient
{
    async fn query(
        &mut self,
        name: Name,
        query_class: DNSClass,
        query_type: RecordType,
    ) -> Result<DnsResponse, ClientError>
    {
        match self {
            DnsClient::Dns(c) => c.query(name, query_class, query_type).await,
            DnsClient::DnsSec(c) => c.query(name, query_class, query_type).await,
        }
    }
}

pub struct Registry
{
    cfg_ate: ConfAte,
    dns: Mutex<DnsClient>,
    chains: Mutex<FxHashMap<String, Arc<MeshClient>>>,
}

impl Registry
{
    pub async fn new(cfg_ate: &ConfAte) -> Arc<Registry>
    {
        debug!("using DNS server: {}", cfg_ate.dns_server);
        let addr: SocketAddr = (cfg_ate.dns_server.clone(), 53).to_socket_addrs().unwrap().next().unwrap();
        
        let (stream, sender)
            = TcpClientStream::<AsyncIoTokioAsStd<TokioTcpStream>>::new(addr);
        let client
            = AsyncClient::new(stream, sender, None);
        let (client, bg)
            = client.await.expect("client failed to connect");
        tokio::spawn(bg);

        let client = MemoizeClientHandle::new(client);
        let dns = match cfg_ate.dns_sec {
            true => {
                debug!("configured for DNSSec");
                DnsClient::DnsSec(DnssecDnsHandle::new(client.clone()))
            },
            false => {
                debug!("configured for plain DNS");
                DnsClient::Dns(client)
            }
        };
        let dns = Mutex::new(dns);
        
        Arc::new(
            Registry {
                cfg_ate: cfg_ate.clone(),
                dns,
                chains: Mutex::new(FxHashMap::default()),
            }
        )
    }

    pub async fn open(&self, url: &Url) -> Result<Arc<MeshSession>, ChainCreationError>
    {
        let mut lock = self.chains.lock().await;
        
        let domain = match url.domain() {
            Some(a) => a.to_string(),
            None => { return Err(ChainCreationError::NoValidDomain(url.to_string())); }
        };
        
        match lock.get(&domain) {
            Some(a) => {
                Ok(a.open(&url).await?)
            },
            None => {
                let cfg_mesh = self.cfg(url).await?;
                let mesh = create_client(&self.cfg_ate, &cfg_mesh).await;
                lock.insert(domain, Arc::clone(&mesh));
                Ok(mesh.open(&url).await?)
            }
        }
    }

    async fn cfg(&self, url: &Url) -> Result<ConfMesh, ChainCreationError>
    {
        if url.scheme().to_lowercase().trim() != "tcp" {
            return Err(ChainCreationError::UnsupportedProtocol);
        }

        let port = match url.port() {
            Some(a) => a,
            None => 5000,
        };

        let mut ret = ConfMesh::default();
        ret.force_listen = None;
        ret.force_client_only = true;

        for n in 0 as i32..10
        {
            // Build the DNS name we will query
            let name_store;
            let name = match url.domain() {
                Some(a) => {
                    match n {
                        0 => a,
                        n => {
                            name_store = format!("{}{}", a, n);
                            name_store.as_str()
                        }
                    }
                },
                None => { return Err(ChainCreationError::NoValidDomain(url.to_string())); }
            };
            
            // Search DNS for entries for this server (Ipv6 takes prioity over Ipv4)
            let (mut addrs, no_more) = self.dns_query(name).await?;
            if addrs.len() <= 0 {
                if n == 0 { debug!("no nodes found for {}", name); }
                break;
            }
            if n > 0 { debug!("another cluster found at {}", name); }

            addrs.sort();
            for addr in addrs.iter() {
                debug!("found node {}", addr);
            }
            
            // Add the cluster to the configuration
            let mut cluster = ConfCluster::default();
            cluster.offset = n;
            for addr in addrs {
                let addr = MeshAddress::new(addr, port);
                cluster.roots.push(addr);
            }
            ret.clusters.push(cluster);

            // If we are not to process any more clusters then break from the loop
            if no_more { break; }
        }

        if ret.clusters.len() <= 0 {
            return Err(ChainCreationError::NoRootFoundForUrl(url.to_string()));
        }

        Ok(ret)
    }

    async fn dns_query(&self, name: &str) -> Result<(Vec<IpAddr>, bool), ClientError>
    {
        match name.to_lowercase().as_str() {
            "localhost" => { return Ok((vec![IpAddr::V4(Ipv4Addr::from_str("127.0.0.1").unwrap())], true)) },
            _ => { }
        };

        if let Ok(ip) = IpAddr::from_str(name) {
            return Ok((vec![ip], true));
        }

        let mut client = self.dns.lock().await;

        let mut addrs = Vec::new();
        let response
            = client.query(Name::from_str(name).unwrap(), DNSClass::IN, RecordType::AAAA).await?;
        for answer in response.answers() {
            if let RData::AAAA(ref address) = *answer.rdata() {
                addrs.push(IpAddr::V6(address.clone()));
            }
        }
        if addrs.len() <= 0 {
            let response
                = client.query(Name::from_str(name).unwrap(), DNSClass::IN, RecordType::A).await?;
            for answer in response.answers() {
                if let RData::A(ref address) = *answer.rdata() {
                    addrs.push(IpAddr::V4(address.clone()));
                }
            }
        }

        Ok((addrs, false))
    }
}