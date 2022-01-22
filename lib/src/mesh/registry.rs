#![allow(unused_imports)]
use async_trait::async_trait;
use error_chain::bail;
use fxhash::FxHashMap;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Mutex as StdMutex;
use std::sync::RwLock as StdRwLock;
use std::time::Duration;
use std::{net::IpAddr, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use crate::chain::Chain;
use crate::chain::ChainKey;
#[cfg(feature = "enable_dns")]
use crate::dns::*;
use crate::engine::TaskEngine;
use crate::error::*;
use crate::loader;
use crate::mesh::*;
use crate::prelude::*;
use crate::service::Service;
use crate::utils::chain_key_16hex;
use crate::{conf::ConfAte, error::ChainCreationError};

pub struct Registry {
    pub cfg_ate: ConfAte,
    #[cfg(feature = "enable_dns")]
    dns: Mutex<DnsClient>,
    pub temporal: bool,
    pub node_id: NodeId,
    pub fail_fast: bool,
    pub keep_alive: Option<Duration>,
    pub ignore_certificates: bool,

    cmd_key: StdMutex<FxHashMap<url::Url, String>>,
    #[cfg(feature = "enable_client")]
    chains: Mutex<FxHashMap<url::Url, Arc<MeshClient>>>,
    pub(crate) services: StdMutex<Vec<Arc<dyn Service>>>,
}

impl Registry {
    pub async fn new(cfg_ate: &ConfAte) -> Registry {
        TaskEngine::run_until(Registry::__new(cfg_ate)).await
    }

    async fn __new(cfg_ate: &ConfAte) -> Registry {
        #[cfg(feature = "enable_dns")]
        let dns = {
            let dns = DnsClient::connect(cfg_ate).await;
            Mutex::new(dns)
        };

        let node_id = NodeId::generate_client_id();
        Registry {
            cfg_ate: cfg_ate.clone(),
            fail_fast: true,
            #[cfg(feature = "enable_dns")]
            dns,
            node_id,
            #[cfg(feature = "enable_local_fs")]
            temporal: cfg_ate.log_path.is_none(),
            #[cfg(not(feature = "enable_local_fs"))]
            temporal: true,
            ignore_certificates: false,
            cmd_key: StdMutex::new(FxHashMap::default()),
            #[cfg(feature = "enable_client")]
            chains: Mutex::new(FxHashMap::default()),
            services: StdMutex::new(Vec::new()),
            keep_alive: None,
        }
    }

    pub fn keep_alive(mut self, duration: Duration) -> Self {
        self.keep_alive = Some(duration);
        self
    }

    pub fn temporal(mut self, temporal: bool) -> Self {
        self.temporal = temporal;
        self
    }

    pub fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    pub fn ignore_certificates(mut self) -> Self {
        self.ignore_certificates = true;
        self
    }

    pub fn cement(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub async fn open(&self, url: &Url, key: &ChainKey) -> Result<ChainGuard, ChainCreationError> {
        TaskEngine::run_until(self.__open(url, key)).await
    }

    pub async fn open_cmd(&self, url: &Url) -> Result<ChainGuard, ChainCreationError> {
        TaskEngine::run_until(async {
            if let Some(a) = self.__try_open(url, &self.chain_key_cmd(url, true)).await? {
                Ok(a)
            } else {
                Ok(self.__open(url, &self.chain_key_cmd(url, false)).await?)
            }
        })
        .await
    }

    async fn __open(&self, url: &Url, key: &ChainKey) -> Result<ChainGuard, ChainCreationError> {
        let loader_local = loader::DummyLoader::default();
        let loader_remote = loader::DummyLoader::default();
        Ok(self
            .__open_ext(url, key, loader_local, loader_remote)
            .await?)
    }

    pub async fn open_ext(
        &self,
        url: &Url,
        key: &ChainKey,
        loader_local: impl loader::Loader + 'static,
        loader_remote: impl loader::Loader + 'static,
    ) -> Result<ChainGuard, ChainCreationError> {
        TaskEngine::run_until(self.__open_ext(url, key, loader_local, loader_remote)).await
    }

    async fn __try_open(
        &self,
        url: &Url,
        key: &ChainKey,
    ) -> Result<Option<ChainGuard>, ChainCreationError> {
        Ok(self.__try_open_ext(url, key).await?)
    }

    #[cfg(feature = "enable_client")]
    async fn __try_open_ext(
        &self,
        url: &Url,
        key: &ChainKey,
    ) -> Result<Option<ChainGuard>, ChainCreationError> {
        let client = {
            let lock = self.chains.lock().await;
            match lock.get(&url) {
                Some(a) => Arc::clone(a),
                None => {
                    trace!("no chain to reuse for chain ({})", key);
                    return Ok(None);
                }
            }
        };

        trace!("trying reuse chain ({}) on mesh client for {}", key, url);

        let ret = client.__try_open_ext(&key).await?;
        let ret = match ret {
            Some(a) => a,
            None => {
                trace!("reuse not possible for chain ({})", key);
                return Ok(None);
            }
        };

        Ok(Some(ChainGuard {
            chain: ret,
            keep_alive: self.keep_alive.clone(),
        }))
    }

    #[cfg(not(feature = "enable_client"))]
    async fn __try_open_ext(
        &self,
        _url: &Url,
        _key: &ChainKey,
    ) -> Result<Option<ChainGuard>, ChainCreationError> {
        return Err(ChainCreationErrorKind::InternalError(
            "client connections are unsupported".to_string(),
        )
        .into());
    }

    #[cfg(feature = "enable_client")]
    async fn __open_ext(
        &self,
        url: &Url,
        key: &ChainKey,
        loader_local: impl loader::Loader + 'static,
        loader_remote: impl loader::Loader + 'static,
    ) -> Result<ChainGuard, ChainCreationError> {
        let client = {
            let mut lock = self.chains.lock().await;
            match lock.get(&url) {
                Some(a) => Arc::clone(a),
                None => {
                    trace!("building mesh client for {}", url);
                    let cfg_mesh = self.cfg_for_url(url).await?;
                    let mesh = MeshClient::new(
                        &self.cfg_ate,
                        &cfg_mesh,
                        self.node_id.clone(),
                        self.temporal,
                    );
                    lock.insert(url.clone(), Arc::clone(&mesh));
                    Arc::clone(&mesh)
                }
            }
        };

        trace!("opening chain ({}) on mesh client for {}", key, url);

        let hello_path = url.path().to_string();
        let ret = client
            .__open_ext(&key, hello_path, loader_local, loader_remote)
            .await?;

        Ok(ChainGuard {
            chain: ret,
            keep_alive: self.keep_alive.clone(),
        })
    }

    #[cfg(not(feature = "enable_client"))]
    async fn __open_ext(
        &self,
        _url: &Url,
        _key: &ChainKey,
        _loader_local: impl loader::Loader + 'static,
        _loader_remote: impl loader::Loader + 'static,
    ) -> Result<ChainGuard, ChainCreationError> {
        return Err(ChainCreationErrorKind::InternalError(
            "client connections are unsupported".to_string(),
        )
        .into());
    }

    pub async fn cfg_for_url(&self, url: &Url) -> Result<ConfMesh, ChainCreationError> {
        let protocol = StreamProtocol::parse(url)?;
        let port = match url.port() {
            Some(a) => a,
            None => protocol.default_port(),
        };
        let domain = match url.domain() {
            Some(a) => a,
            None => {
                bail!(ChainCreationErrorKind::NoValidDomain(url.to_string()));
            }
        };

        let mut ret = self.cfg_for_domain(domain, port).await?;
        ret.remote = url.clone();
        ret.wire_protocol = protocol;

        // Set the fail fast
        ret.fail_fast = self.fail_fast;

        // Set the ignore certificates
        if self.ignore_certificates {
            ret.certificate_validation = CertificateValidation::AllowAll;
        }

        // Add all the global certificates
        if let CertificateValidation::AllowedCertificates(allowed) = &mut ret.certificate_validation
        {
            for cert in GLOBAL_CERTIFICATES.read().unwrap().iter() {
                allowed.push(cert.clone());
            }
        }

        // Perform a DNS query on the domain and pull down TXT records
        #[cfg(feature = "enable_dns")]
        if let CertificateValidation::AllowedCertificates(allowed) = &mut ret.certificate_validation
        {
            let mut certs = self.dns_certs(domain).await?;
            allowed.append(&mut certs);
        }

        Ok(ret)
    }

    async fn cfg_roots(
        &self,
        domain: &str,
        port: u16,
    ) -> Result<Vec<MeshAddress>, ChainCreationError> {
        let mut roots = Vec::new();

        // Search DNS for entries for this server (Ipv6 takes prioity over Ipv4)
        #[cfg(feature = "enable_dns")]
        {
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
                roots.push(addr);
            }
        };
        #[cfg(not(feature = "enable_dns"))]
        {
            let addr = MeshAddress::new(domain, port);
            roots.push(addr);
        }

        if roots.len() <= 0 {
            bail!(ChainCreationErrorKind::NoRootFoundForDomain(
                domain.to_string()
            ));
        }

        Ok(roots)
    }

    #[cfg(feature = "enable_dns")]
    pub async fn dns_certs(&self, name: &str) -> Result<Vec<AteHash>, ClientError> {
        match name.to_lowercase().as_str() {
            "localhost" => {
                return Ok(Vec::new());
            }
            _ => {}
        };

        if let Ok(_) = IpAddr::from_str(name) {
            return Ok(Vec::new());
        }

        trace!("dns_query for {}", name);
        let mut client = self.dns.lock().await;

        let mut txts = Vec::new();
        if let Some(response) = client
            .query(Name::from_str(name).unwrap(), DNSClass::IN, RecordType::TXT)
            .await
            .ok()
        {
            for answer in response.answers() {
                if let RData::TXT(ref txt) = *answer.rdata() {
                    txts.push(txt.to_string());
                }
            }
        }

        let prefix = "ate-cert-";

        let mut certs = Vec::new();
        for txt in txts {
            let txt = txt.replace(" ", "");
            if txt.trim().starts_with(prefix) {
                let start = prefix.len();
                let hash = &txt.trim()[start..];
                if let Some(hash) = AteHash::from_hex_string(hash) {
                    trace!("found certificate({}) for {}", hash, name);
                    certs.push(hash);
                }
            }
        }
        trace!(
            "dns_query for {} returned {} certificates",
            name,
            certs.len()
        );

        Ok(certs)
    }

    #[cfg(feature = "enable_dns")]
    pub async fn dns_query(&self, name: &str) -> Result<Vec<IpAddr>, ClientError> {
        match name.to_lowercase().as_str() {
            "localhost" => return Ok(vec![IpAddr::V4(Ipv4Addr::from_str("127.0.0.1").unwrap())]),
            _ => {}
        };

        if let Ok(ip) = IpAddr::from_str(name) {
            return Ok(vec![ip]);
        }

        trace!("dns_query for {}", name);
        let mut client = self.dns.lock().await;

        let mut addrs = Vec::new();
        if let Some(response) = client
            .query(
                Name::from_str(name).unwrap(),
                DNSClass::IN,
                RecordType::AAAA,
            )
            .await
            .ok()
        {
            for answer in response.answers() {
                if let RData::AAAA(ref address) = *answer.rdata() {
                    addrs.push(IpAddr::V6(address.clone()));
                }
            }
        }
        if addrs.len() <= 0 {
            let response = client
                .query(Name::from_str(name).unwrap(), DNSClass::IN, RecordType::A)
                .await?;
            for answer in response.answers() {
                if let RData::A(ref address) = *answer.rdata() {
                    addrs.push(IpAddr::V4(address.clone()));
                }
            }
        }
        trace!("dns_query for {} returned {} addresses", name, addrs.len());

        Ok(addrs)
    }

    pub(crate) async fn cfg_for_domain(
        &self,
        domain_name: &str,
        port: u16,
    ) -> Result<ConfMesh, ChainCreationError> {
        let roots = self.cfg_roots(domain_name, port).await?;
        let remote = url::Url::parse(format!("{}://{}", Self::guess_schema(port), domain_name).as_str())?;
        let ret = ConfMesh::new(domain_name, remote, roots.iter());
        Ok(ret)
    }

    pub fn guess_schema(port: u16) -> &'static str {
        match port {
            80 => "ws",
            443 => "wss",
            _ => "tcp"
        }
    }

    /// Will generate a random command key - reused for 30 seconds to improve performance
    /// (note: this cache time must be less than the server cache time on commands)
    fn chain_key_cmd(&self, url: &url::Url, reuse: bool) -> ChainKey {
        let mut guard = self.cmd_key.lock().unwrap();
        if reuse {
            if let Some(hex) = guard.get(url) {
                return chain_key_16hex(hex.as_str(), Some("cmd"));
            }
        }

        let hex = AteHash::generate().to_hex_string();
        guard.insert(url.clone(), hex.clone());
        chain_key_16hex(hex.as_str(), Some("cmd"))
    }
}

#[derive(Clone)]
pub struct ChainGuard {
    keep_alive: Option<Duration>,
    chain: Arc<Chain>,
}

impl ChainGuard {
    pub fn as_ref(&self) -> &Chain {
        self.chain.deref()
    }

    pub fn as_arc(&self) -> Arc<Chain> {
        Arc::clone(&self.chain)
    }

    pub async fn dio(&self, session: &'_ dyn AteSession) -> Arc<Dio> {
        self.chain.dio(session).await
    }

    /// Opens a data access layer that allows mutable changes to data.
    /// Transaction consistency on commit will be guarranted for local redo log files
    pub async fn dio_mut(&self, session: &'_ dyn AteSession) -> Arc<DioMut> {
        self.chain.dio_mut(session).await
    }

    /// Opens a data access layer that allows mutable changes to data (in a fire-and-forget mode).
    /// No transaction consistency on commits will be enforced
    pub async fn dio_fire(&self, session: &'_ dyn AteSession) -> Arc<DioMut> {
        self.chain.dio_fire(session).await
    }

    /// Opens a data access layer that allows mutable changes to data.
    /// Transaction consistency on commit will be guarranted for all remote replicas
    pub async fn dio_full(&self, session: &'_ dyn AteSession) -> Arc<DioMut> {
        self.chain.dio_full(session).await
    }

    /// Opens a data access layer that allows mutable changes to data.
    /// Transaction consistency on commit must be specified
    pub async fn dio_trans(
        &self,
        session: &'_ dyn AteSession,
        scope: TransactionScope,
    ) -> Arc<DioMut> {
        self.chain.dio_trans(session, scope).await
    }

    pub async fn invoke<REQ, RES, ERR>(&self, request: REQ) -> Result<Result<RES, ERR>, InvokeError>
    where
        REQ: Clone + Serialize + DeserializeOwned + Sync + Send + ?Sized,
        RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
        ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized,
    {
        self.as_arc().invoke(request).await
    }

    pub async fn invoke_ext<REQ, RES, ERR>(
        &self,
        session: Option<&'_ dyn AteSession>,
        request: REQ,
        timeout: Duration,
    ) -> Result<Result<RES, ERR>, InvokeError>
    where
        REQ: Clone + Serialize + DeserializeOwned + Sync + Send + ?Sized,
        RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
        ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized,
    {
        self.as_arc().invoke_ext(session, request, timeout).await
    }
}

impl Deref for ChainGuard {
    type Target = Chain;

    fn deref(&self) -> &Self::Target {
        self.chain.deref()
    }
}

impl Drop for ChainGuard {
    fn drop(&mut self) {
        if let Some(duration) = &self.keep_alive {
            let chain = Arc::clone(&self.chain);
            let duration = duration.clone();
            TaskEngine::spawn(async move {
                trace!("keep-alive: warm down for {}", chain.key());
                crate::engine::sleep(duration).await;

                // If we are the last then do a cleanup
                if Arc::strong_count(&chain) <= 1 {
                    let ret = chain.shutdown().await;
                    if let Err(err) = ret {
                        error!("shutdown failed during guard drop - {}", err);
                    }
                }
                drop(chain);
            });
        }
    }
}
