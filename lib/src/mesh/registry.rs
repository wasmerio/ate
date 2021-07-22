#![allow(unused_imports)]
use log::{warn, debug, error};
use async_trait::async_trait;
use crate::{
    conf::ConfAte,
    error::ChainCreationError
};
use crate::chain::Chain;
use crate::chain::ChainKey;
use crate::mesh::*;
use crate::error::*;
use crate::loader;
use crate::repository::ChainRepository;
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
    Dns {
        cfg: ConfAte,
        client: MemoizeClientHandle<AsyncClient>
    },
    DnsSec {
        cfg: ConfAte,
        client: DnssecDnsHandle<MemoizeClientHandle<AsyncClient>>
    }
}

impl DnsClient
{
    async fn connect(cfg: &ConfAte) -> DnsClient
    {
        debug!("using DNS server: {}", cfg.dns_server);
        let addr: SocketAddr = (cfg.dns_server.clone(), 53).to_socket_addrs().unwrap().next().unwrap();
        
        let (stream, sender)
            = TcpClientStream::<AsyncIoTokioAsStd<TokioTcpStream>>::new(addr);
        let client
            = AsyncClient::new(stream, sender, None);
        let (client, bg)
            = client.await.expect("client failed to connect");
        tokio::spawn(bg);

        let client = MemoizeClientHandle::new(client);

        match cfg.dns_sec {
            false => {
                debug!("configured for DNSSec");
                DnsClient::Dns {
                    cfg: cfg.clone(),
                    client
                }
            },
            true => {
                debug!("configured for plain DNS");
                DnsClient::DnsSec {
                    cfg: cfg.clone(),
                    client: DnssecDnsHandle::new(client.clone())
                }
            }
        }
    }

    async fn reconnect(&mut self)
    {
        let cfg = match self {
            DnsClient::Dns { cfg, client: _} => cfg.clone(),
            DnsClient::DnsSec { cfg, client: _} => cfg.clone()
        };

        *self = DnsClient::connect(&cfg).await;
    }

    async fn query(
        &mut self,
        name: Name,
        query_class: DNSClass,
        query_type: RecordType,
    ) -> Result<DnsResponse, ClientError>
    {
        let ret = {
            match self {
                DnsClient::Dns{cfg: _, client: c} => c.query(name.clone(), query_class, query_type).await,
                DnsClient::DnsSec{cfg: _, client: c} => c.query(name.clone(), query_class, query_type).await,
            }
        };

        match ret {
            Ok(a) => Ok(a),
            Err(_) => {
                self.reconnect().await;

                match self {
                    DnsClient::Dns{cfg: _, client: c} => c.query(name, query_class, query_type).await,
                    DnsClient::DnsSec{cfg: _, client: c} => c.query(name, query_class, query_type).await,
                }
            }
        }
    }
}

pub struct Registry
{
    pub cfg_ate: ConfAte,
    dns: Mutex<DnsClient>,
    pub temporal: bool,
    pub fail_fast: bool,
    chains: Mutex<FxHashMap<url::Url, Arc<MeshClient>>>,
}

impl Registry
{
    pub async fn new(cfg_ate: &ConfAte) -> Registry
    {
        let dns = DnsClient::connect(cfg_ate).await;
        let dns = Mutex::new(dns);
        
        Registry {
            cfg_ate: cfg_ate.clone(),
            fail_fast: true,
            dns,
            temporal: true,
            chains: Mutex::new(FxHashMap::default()),
        }
    }

    pub fn temporal(mut self, temporal: bool) -> Self
    {
        self.temporal = temporal;
        self
    }

    pub fn fail_fast(mut self, fail_fast: bool) -> Self
    {
        self.fail_fast = fail_fast;
        self
    }

    pub fn cement(self) -> Arc<Self>
    {
        Arc::new(self)
    }

    pub async fn open_ext(&self, url: &Url, key: &ChainKey, loader_local: Box<impl loader::Loader>, loader_remote: Box<impl loader::Loader>) -> Result<Arc<Chain>, ChainCreationError>
    {
        let mut lock = self.chains.lock().await;
        
        let hello_path = url.path().to_string();

        match lock.get(&url) {
            Some(a) => {
                Ok(a.open_ext(&key, hello_path, loader_local, loader_remote).await?)
            },
            None => {
                let cfg_mesh = self.cfg(url).await?;
                let mesh = create_client(&self.cfg_ate, &cfg_mesh, self.temporal).await;
                lock.insert(url.clone(), Arc::clone(&mesh));
                Ok(mesh.open_ext(&key, hello_path, loader_local, loader_remote).await?)
            }
        }
    }

    async fn cfg(&self, url: &Url) -> Result<ConfMesh, ChainCreationError>
    {
        let protocol = StreamProtocol::parse(url)?;
        let port = match url.port() {
            Some(a) => a,
            None => protocol.default_port(),
        };
        let domain = match url.domain() {
            Some(a) => a,
            None => { return Err(ChainCreationError::NoValidDomain(url.to_string())); }
        };

        let mut ret = ConfMesh::for_domain(domain.to_string());
        ret.wire_protocol = protocol;
        
        // Set the fail fast
        ret.fail_fast = self.fail_fast;

        // Search DNS for entries for this server (Ipv6 takes prioity over Ipv4)
        let mut addrs = self.dns_query(domain).await?;
        if addrs.len() <= 0 {
            debug!("no nodes found for {}", domain);
        }

        addrs.sort();
        for addr in addrs.iter() {
            debug!("found node {}", addr);
        }
        
        // Add the cluster to the configuration
        for addr in addrs {
            let addr = MeshAddress::new(addr, port);
            ret.roots.push(addr);
        }

        if ret.roots.len() <= 0 {
            return Err(ChainCreationError::NoRootFoundForUrl(url.to_string()));
        }

        Ok(ret)
    }

    async fn dns_query(&self, name: &str) -> Result<Vec<IpAddr>, ClientError>
    {
        match name.to_lowercase().as_str() {
            "localhost" => { return Ok(vec![IpAddr::V4(Ipv4Addr::from_str("127.0.0.1").unwrap())]) },
            _ => { }
        };

        if let Ok(ip) = IpAddr::from_str(name) {
            return Ok(vec![ip]);
        }

        let mut client = self.dns.lock().await;

        let mut addrs = Vec::new();
        if let Some(response)
            = client.query(Name::from_str(name).unwrap(), DNSClass::IN, RecordType::AAAA).await.ok()
        {
            for answer in response.answers() {
                if let RData::AAAA(ref address) = *answer.rdata() {
                    addrs.push(IpAddr::V6(address.clone()));
                }
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

        Ok(addrs)
    }
}

#[async_trait]
impl ChainRepository
for Registry
{
    async fn open(self: Arc<Self>, url: &Url, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let loader_local = Box::new(loader::DummyLoader::default());
        let loader_remote = Box::new(loader::DummyLoader::default());

        let weak = Arc::downgrade(&self);
        let ret = self.open_ext(url, key, loader_local, loader_remote).await?;
        ret.inside_sync.write().repository = Some(weak);
        Ok(ret)
    }
}