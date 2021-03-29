#![allow(unused_imports)]
use crate::{
    conf::Config,
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

pub struct Registry
{
    cfg: Config,
    dns: Mutex<DnssecDnsHandle<MemoizeClientHandle<AsyncClient>>>,
    chains: Mutex<FxHashMap<String, Arc<dyn Mesh>>>,
}

impl Registry
{
    pub async fn new(cfg: Config) -> Arc<Registry>
    {
        let addr: SocketAddr = ("8.8.8.8", 53).to_socket_addrs().unwrap().next().unwrap();
        let (stream, sender)
            = TcpClientStream::<AsyncIoTokioAsStd<TokioTcpStream>>::new(addr);
        let client
            = AsyncClient::new(stream, sender, None);
        let (client, bg)
            = client.await.expect("client failed to connect");
        tokio::spawn(bg);

        let client = MemoizeClientHandle::new(client);
        let client = DnssecDnsHandle::new(client);
        
        Arc::new(
            Registry {
                cfg,
                dns: Mutex::new(client),
                chains: Mutex::new(FxHashMap::default()),
            }
        )
    }

    pub async fn chain(&self, url: &Url) -> Result<Arc<Chain>, ChainCreationError>
    {
        let key = ChainKey::new(url.path().to_string());
        let mut lock = self.chains.lock().await;
        
        let domain = match url.domain() {
            Some(a) => a.to_string(),
            None => { return Err(ChainCreationError::NoValidDomain(url.to_string())); }
        };
        
        match lock.get(&domain) {
            Some(a) => {
                Ok(a.open(key).await?)
            },
            None => {
                let cfg = self.cfg(url).await?;
                let mesh = create_mesh(&cfg).await;
                lock.insert(domain, Arc::clone(&mesh));
                Ok(mesh.open(key).await?)
            }
        }
    }

    async fn cfg(&self, url: &Url) -> Result<Config, ChainCreationError>
    {
        let mut client = self.dns.lock().await;

        if url.scheme().to_lowercase().trim() != "tcp" {
            return Err(ChainCreationError::UnsupportedProtocol);
        }

        let port = match url.port() {
            Some(a) => a,
            None => 5000,
        };

        let mut ret = self.cfg.clone();
        for n in 0 as i32.. {
            let name_store;
            let name = Name::from_str(
                match url.domain() {
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
                }
            ).unwrap();
            
            let mut addrs = Vec::new();
            
            let response
                = client.query(name.clone(), DNSClass::IN, RecordType::AAAA).await?;
            for answer in response.answers() {
                if let RData::AAAA(ref address) = *answer.rdata() {
                    addrs.push(IpAddr::V6(address.clone()));
                }
            }
            let response
                = client.query(name.clone(), DNSClass::IN, RecordType::A).await?;
            for answer in response.answers() {
                if let RData::A(ref address) = *answer.rdata() {
                    addrs.push(IpAddr::V4(address.clone()));
                }
            }

            if addrs.len() <= 0 {
                break;
            }
            
            let mut cluster = ConfCluster::default();
            for addr in addrs {
                let addr = MeshAddress::new(addr, port);
                cluster.roots.push(addr);
            }
            ret.clusters.push(cluster);
        }

        if ret.clusters.len() <= 0 {
            return Err(ChainCreationError::NoRootFoundForUrl(url.to_string()));
        }

        Ok(ret)
    }
}