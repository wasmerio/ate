#![allow(unused_imports)]
use log::{info, error, debug};
use async_trait::async_trait;

use serde::{Serialize, Deserialize};
use crate::{anti_replay::AntiReplayPlugin, chain::Chain, time::TimestampEnforcer, tree::{TreeAuthorityPlugin, TreeCompactor}};
use std::{net::IpAddr, str::FromStr};
use std::sync::Arc;
use url::Url;

use super::validator::*;
use super::compact::*;
use super::index::*;
use super::lint::*;
use super::transform::*;
use super::plugin::*;
use super::trust::ChainKey;
use super::trust::IntegrityMode;
use super::crypto::PublicSignKey;
use super::crypto::KeySize;
use super::error::*;
use super::crypto::Hash;
use super::spec::*;
use super::pipe::*;
use super::session::Session;
use super::repository::ChainRepository;

/// Represents a target node within a mesh
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeshAddress
{
    pub ip: IpAddr,
    pub port: u16,
}

impl MeshAddress
{
    #[allow(dead_code)]
    pub fn new(ip: IpAddr, port: u16) -> MeshAddress {
        MeshAddress {
            ip: ip,
            port,
        }
    }

    pub fn hash(&self) -> Hash {
        match self.ip {
            IpAddr::V4(ip) => {
                Hash::from_bytes_twice(&ip.octets(), &self.port.to_be_bytes())
            },
            IpAddr::V6(ip) => {
                Hash::from_bytes_twice(&ip.octets(), &self.port.to_be_bytes())
            }
        }
    }
}

/// Represents all nodes within this cluster. All the chains
/// are spread evenly across the nodes within a cluster using a hashing
/// algorithm. Care must be taken when added new addresses that the
/// redo logs are not lost during a respreading of addresses. The recommended
/// way to grow clusters is to first run an ATE mirror on the new cluster
/// nodes then once its running switch to an active cluster
#[derive(Debug, Clone)]
pub struct ConfMesh
{
    /// List of all the addresses that the root nodes exists on
    pub roots: Vec<MeshAddress>,
    /// Forces ATE to act as a client even if its local IP address is one
    /// of the node machines in the clusters (normally ATE would automatically
    /// listen for connections)
    pub force_client_only: bool,
    /// Forces ATE to listen on a particular address for connections even if
    /// the address is not in the list of cluster nodes.
    pub force_listen: Option<MeshAddress>,
}

impl ConfMesh
{
    /// Represents a single server listening on all available addresses. All chains
    /// will be stored locally to this server and there is no replication
    pub fn solo(addr: &str, port: u16) -> ConfMesh
    {
        let mut cfg_mesh = ConfMesh::default();
        let addr = MeshAddress::new(IpAddr::from_str(addr).unwrap(), port);
        cfg_mesh.roots.push(addr.clone());
        cfg_mesh.force_listen = Some(addr);
        cfg_mesh
    }
}

/// Configuration settings for the ATE datastore
///
#[derive(Debug, Clone)]
pub struct ConfAte
{
    /// Optimizes ATE for a specific group of usecases
    configured_for: ConfiguredFor,

    /// Directory path that the redo logs will be stored.
    pub log_path: String,

    /// NTP pool server which ATE will synchronize its clocks with, its
    /// important to have synchronized clocks with ATE as it uses time as
    /// digest to prevent replay attacks
    pub ntp_pool: String,
    /// Port that the NTP server is listening on (defaults to 123)
    pub ntp_port: u16,
    /// Flag that indicates if the time keeper will sync with NTP or not
    /// (avoiding NTP sync means one can run fully offline but time drift
    ///  will cause issues with multi factor authentication and timestamps)
    pub ntp_sync: bool,

    /// Flag that determines if ATE will use DNSSec or just plain DNS
    pub dns_sec: bool,
    /// DNS server that queries will be made do by the chain registry
    pub dns_server: String,

    /// Flag that indicates if encryption will be used for the underlying
    /// connections over the wire. When using a ATE's in built encryption
    /// and quantum resistant signatures it is not mandatory to use
    /// wire encryption as confidentially and integrity are already enforced however
    /// for best security it is advisable to apply a layered defence, of
    /// which double encrypting your data and the metadata around it is
    /// another defence.
    pub wire_encryption: Option<KeySize>,

    /// Size of the buffer on mesh clients, tweak this number with care
    pub buffer_size_client: usize,
    /// Size of the buffer on mesh servers, tweak this number with care
    pub buffer_size_server: usize,

    /// Size of the local cache that stores redo log entries in memory
    pub load_cache_size: usize,
    /// Number of seconds that redo log entries will remain in memory before
    /// they are evicted
    pub load_cache_ttl: u64,

    /// Serialization format of the log files
    pub log_format: MessageFormat,
    /// Serialization format of the data on the network pipes between nodes and clients
    pub wire_format: SerializationFormat,
}

impl ConfAte
{
    pub fn configured_for(&mut self, configured_for: ConfiguredFor)
    {
        self.configured_for = configured_for;

        match configured_for {
            ConfiguredFor::BestPerformance => {
                self.log_format.meta = SerializationFormat::Bincode;
                self.log_format.data = SerializationFormat::Bincode;
            },
            ConfiguredFor::BestCompatibility => {
                self.log_format.meta = SerializationFormat::Json;
                self.log_format.data = SerializationFormat::Json;
            },
            _ => {
                self.log_format.meta = SerializationFormat::Bincode;
                self.log_format.data = SerializationFormat::Json;
            }
        }
    }
}

impl Default
for ConfMesh
{
    fn default() -> ConfMesh {
        ConfMesh {
            roots: Vec::new(),
            force_client_only: false,
            force_listen: None,
        }
    }
}

impl Default
for ConfAte
{
    fn default() -> ConfAte {
        ConfAte {
            log_path: "/tmp/ate".to_string(),
            dns_sec: false,
            dns_server: "8.8.8.8".to_string(),
            ntp_sync: true,
            ntp_pool: "pool.ntp.org".to_string(),
            ntp_port: 123,
            wire_encryption: Some(KeySize::Bit128),
            configured_for: ConfiguredFor::default(),
            buffer_size_client: 2,
            buffer_size_server: 10,
            load_cache_size: 1000,
            load_cache_ttl: 30,
            log_format: MessageFormat {
                meta: SerializationFormat::Bincode,
                data: SerializationFormat::Json,
            },
            wire_format: SerializationFormat::Bincode,
        }
    }
}

#[cfg(test)]
pub(crate) fn mock_test_config() -> ConfAte {
    let mut ret = ConfAte::default();
    ret.log_path = "/tmp/ate".to_string();
    return ret;
}

#[cfg(test)]
pub(crate) fn mock_test_mesh() -> ConfMesh {
    let mut ret = ConfMesh::default();
    ret.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), 4001));
    ret
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HashRoutine
{
    Sha3,
    Blake3,
}

/// Determines what optimizes and defaults ATE selects based of a particular
/// group of usecases
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfiguredFor
{
    /// ATE is left completely unconfigured with no-assumptions and no default functionality
    Raw,
    /// ATE is configured with the minimum that is considered at least functional
    Barebone,
    /// ATE will optimize its usage for the redo-logs with the smallest size possible, this
    /// includes using compression on the data streams by default.
    SmallestSize,
    /// ATE will use serializers that are much faster than normal however they do not support
    /// forward or backwards compatibility meaning changes to the data object schemas will
    /// break your trees thus you will need to handle versioning yourself manually.
    BestPerformance,
    /// ATE will use serializers that provide both forward and backward compatibility for changes
    /// to the metadata schema and the data schema. This format while slower than the performance
    /// setting allows seamless upgrades and changes to your model without breaking existing data.
    BestCompatibility,
    /// A balance between performance, compatibility and security that gives a bit of each without
    /// without going towards the extremes of any. For instance, the data model is forwards and
    /// backwards compatible however the metadata is not. Encryption is good eno\for all known
    /// attacks of today but less protected against unknown attacks of the future.
    Balanced,
    /// Provides the best encryption routines available at the expense of performance and size
    BestSecurity,
}

impl std::str::FromStr
for ConfiguredFor
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "raw" => Ok(ConfiguredFor::Raw),
            "barebone" => Ok(ConfiguredFor::Barebone),
            "best_performance" => Ok(ConfiguredFor::BestPerformance),
            "performance" => Ok(ConfiguredFor::BestPerformance),
            "speed" => Ok(ConfiguredFor::BestPerformance),
            "best_compatibility" => Ok(ConfiguredFor::BestCompatibility),
            "compatibility" => Ok(ConfiguredFor::BestCompatibility),
            "balanced" => Ok(ConfiguredFor::Balanced),
            "best_security" => Ok(ConfiguredFor::BestSecurity),
            "security" => Ok(ConfiguredFor::BestSecurity),
            _ => Err("valid values are 'raw', 'barebone', 'best_performance', 'performance', 'speed', 'best_compatibility', 'compatibility', 'balanced', 'best_security' and 'security'"),
        }
    }
}

impl Default
for ConfiguredFor
{
    fn default() -> ConfiguredFor {
        ConfiguredFor::Balanced
    }
}

/// Building class used to construct a chain-of-trust with
/// its user defined plugins and configuration. Nearly always
/// this builder will be used to create and load your chains.
pub struct ChainOfTrustBuilder
{
    pub(super) cfg: ConfAte, 
    pub(super) configured_for: ConfiguredFor,
    pub(super) validators: Vec<Box<dyn EventValidator>>,
    pub(super) compactors: Vec<Box<dyn EventCompactor>>,
    pub(super) linters: Vec<Box<dyn EventMetadataLinter>>,
    pub(super) transformers: Vec<Box<dyn EventDataTransformer>>,
    pub(super) indexers: Vec<Box<dyn EventIndexer>>,
    pub(super) plugins: Vec<Box<dyn EventPlugin>>,
    pub(super) pipes: Option<Arc<Box<dyn EventPipe>>>,
    pub(super) tree: Option<TreeAuthorityPlugin>,
    pub(super) truncate: bool,
    pub(super) temporal: bool,
    pub(super) integrity: IntegrityMode,
    pub(super) session: Session,
}

impl Clone
for ChainOfTrustBuilder
{
    fn clone(&self) -> Self {
        ChainOfTrustBuilder {
            cfg: self.cfg.clone(),
            configured_for: self.configured_for.clone(),
            validators: self.validators.iter().map(|a| a.clone_validator()).collect::<Vec<_>>(),
            compactors: self.compactors.iter().map(|a| a.clone_compactor()).collect::<Vec<_>>(),
            linters: self.linters.iter().map(|a| a.clone_linter()).collect::<Vec<_>>(),
            transformers: self.transformers.iter().map(|a| a.clone_transformer()).collect::<Vec<_>>(),
            indexers: self.indexers.iter().map(|a| a.clone_indexer()).collect::<Vec<_>>(),
            plugins: self.plugins.iter().map(|a| a.clone_plugin()).collect::<Vec<_>>(),
            pipes: self.pipes.clone(),
            tree: self.tree.clone(),
            session: self.session.clone(),
            truncate: self.truncate,
            temporal: self.temporal,
            integrity: self.integrity,
        }
    }
}

impl ChainOfTrustBuilder
{
    #[allow(dead_code)]
    pub async fn new(cfg: &ConfAte) -> ChainOfTrustBuilder {
        ChainOfTrustBuilder {
            cfg: cfg.clone(),
            configured_for: cfg.configured_for.clone(),
            validators: Vec::new(),
            indexers: Vec::new(),
            compactors: Vec::new(),
            linters: Vec::new(),
            transformers: Vec::new(),
            plugins: Vec::new(),
            pipes: None,
            tree: None,
            session: Session::new(&cfg),
            truncate: false,
            temporal: false,
            integrity: IntegrityMode::Distributed,
        }
        .with_defaults()
        .await
    }

    #[allow(dead_code)]
    pub async fn with_defaults(mut self) -> Self {
        self.validators.clear();
        self.indexers.clear();
        self.linters.clear();
        self.transformers.clear();
        self.plugins.clear();
        self.compactors.clear();
        self.tree = None;
        self.truncate = false;

        if self.configured_for == ConfiguredFor::Raw {
            return self;
        }

        self.compactors.push(Box::new(RemoveDuplicatesCompactor::default()));
        self.compactors.push(Box::new(TombstoneCompactor::default()));
        self.plugins.push(Box::new(AntiReplayPlugin::default()));

        self.cfg.wire_encryption = None;
        match self.configured_for {
            ConfiguredFor::SmallestSize => {
                self.transformers.insert(0, Box::new(CompressorWithSnapTransformer::default()));
            },
            ConfiguredFor::Balanced => {
                self.cfg.wire_encryption = Some(KeySize::Bit128);
            },
            ConfiguredFor::BestSecurity => {
                self.cfg.dns_sec = true;
                self.cfg.wire_encryption = Some(KeySize::Bit256);
            }
            _ => {}
        }

        if self.configured_for == ConfiguredFor::Barebone {
            self.validators.push(Box::new(RubberStampValidator::default()));
            return self;
        }
        else
        {
            self.tree = Some(super::tree::TreeAuthorityPlugin::new());
            self.compactors.push(Box::new(TreeCompactor::default()));

            let tolerance = match self.configured_for {
                ConfiguredFor::BestPerformance => 4000,
                ConfiguredFor::BestSecurity => 1000,
                _ => 2000,
            };
            self.plugins.push(Box::new(TimestampEnforcer::new(&self.cfg, tolerance).await.unwrap()));
        }

        self
    }

    #[allow(dead_code)]
    pub fn without_defaults(mut self) -> Self {
        self.validators.clear();
        self.indexers.clear();
        self.compactors.clear();
        self.linters.clear();
        self.transformers.clear();
        self.plugins.clear();
        self.tree = None;
        self.truncate = false;
        self
    }

    #[allow(dead_code)]
    pub fn add_compactor(mut self, compactor: Box<dyn EventCompactor>) -> Self {
        self.compactors.push(compactor);
        self
    }

    #[allow(dead_code)]
    pub fn add_validator(mut self, validator: Box<dyn EventValidator>) -> Self {
        self.validators.push(validator);
        self
    }
    #[allow(dead_code)]
    pub fn add_metadata_linter(mut self, linter: Box<dyn EventMetadataLinter>) -> Self {
        self.linters.push(linter);
        self
    }

    #[allow(dead_code)]
    pub fn add_data_transformer(mut self, transformer: Box<dyn EventDataTransformer>) -> Self {
        self.transformers.push(transformer);
        self
    }

    #[allow(dead_code)]
    pub fn add_indexer(mut self, indexer: Box<dyn EventIndexer>) -> Self {
        self.indexers.push(indexer);
        self
    }


    #[allow(dead_code)]
    pub fn add_plugin(mut self, plugin: Box<dyn EventPlugin>) -> Self {
        self.plugins.push(plugin);
        self
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(mut self, key: &PublicSignKey) -> Self {
        if let Some(tree) = &mut self.tree {
            tree.add_root_public_key(key);
        }
        self
    }

    #[allow(dead_code)]
    pub(crate) fn add_pipe(mut self, mut pipe: Box<dyn EventPipe>) -> Self {
        let next = self.pipes.take();
        if let Some(next) = next {
            pipe.set_next(next);
        }
        self.pipes = Some(Arc::new(pipe));
        self
    }

    #[allow(dead_code)]
    pub fn set_session(mut self, session: Session) -> Self {
        self.session = session;
        self
    }

    #[allow(dead_code)]
    pub fn truncate(mut self, val: bool) -> Self {
        self.truncate = val;
        self
    }

    #[allow(dead_code)]
    pub fn temporal(mut self, val: bool) -> Self {
        self.temporal = val;
        self
    }

    #[allow(dead_code)]
    pub fn integrity(mut self, mode: IntegrityMode) -> Self {
        self.integrity = mode;
        self
    }

    #[allow(dead_code)]
    pub fn build
    (
        self,
    )
    -> Arc<ChainOfTrustBuilder>
    {
        Arc::new(self)
    }
    
    #[allow(dead_code)]
    pub async fn open_by_url(self: &Arc<Self>, url: &Url) -> Result<Arc<Chain>, ChainCreationError>
    {
        let repo = Arc::clone(self);
        let repo: Arc<dyn ChainRepository> = repo;
        repo.open_by_url(url).await
    }

    #[allow(dead_code)]
    pub async fn open_by_key(self: &Arc<Self>, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let repo = Arc::clone(self);
        let repo: Arc<dyn ChainRepository> = repo;
        repo.open_by_key(key).await
    }

    #[allow(dead_code)]
    pub async fn open(self: &Arc<Self>, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        self.open_by_key(key).await
    }
}

#[async_trait]
impl ChainRepository
for ChainOfTrustBuilder
{
    async fn open_by_url(self: Arc<Self>, url: &Url) -> Result<Arc<Chain>, ChainCreationError>
    {
        let key = ChainKey::from_url(url);
        self.open_by_key(&key).await
    }

    async fn open_by_key(self: Arc<Self>, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let weak = Arc::downgrade(&self);
        let ret = Arc::new(Chain::new((*self).clone(), key).await?);
        ret.inside_sync.write().repository = Some(weak);
        Ok(ret)
    }
}

#[test]
fn test_config_mocking() {
    crate::utils::bootstrap_env();

    let cfg = mock_test_mesh();
    assert_eq!(cfg.roots.iter().next().unwrap().ip.to_string(), "127.0.0.1");
}