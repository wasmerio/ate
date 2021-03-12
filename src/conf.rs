use serde::{Serialize, Deserialize};
use crate::{accessor::ChainAccessor, time::TimestampEnforcer, tree::TreeAuthorityPlugin};
#[allow(unused_imports)]
use std::{net::IpAddr, str::FromStr};

use super::validator::*;
use super::compact::*;
use super::index::*;
use super::lint::*;
use super::transform::*;
use super::plugin::*;
use super::chain::ChainKey;
use super::crypto::PublicKey;
use super::error::*;
use super::crypto::Hash;

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

#[derive(Debug, Clone)]
pub struct Config
{
    pub log_path: String,
    pub log_temp: bool,

    pub ntp_pool: String,
    pub ntp_port: u32,

    pub roots: Vec<MeshAddress>,
    pub force_client_only: bool,
    pub force_listen: Option<MeshAddress>,

    pub configured_for: ConfiguredFor,

    pub buffer_size_client: usize,
    pub buffer_size_server: usize,
}

impl Default
for Config
{
    fn default() -> Config {
        Config {
            log_path: "/tmp/ate".to_string(),
            log_temp: true,
            ntp_pool: "pool.ntp.org".to_string(),
            ntp_port: 123,
            roots: Vec::new(),
            force_client_only: false,
            force_listen: None,
            configured_for: ConfiguredFor::default(),
            buffer_size_client: 1000,
            buffer_size_server: 1000,
        }
    }
}

#[cfg(test)]
pub fn mock_test_config() -> Config {
    let mut ret = Config::default();
    ret.log_path = "/tmp/ate".to_string();
    ret.log_temp = true;
    ret.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), 4001));
    return ret;
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ConfiguredFor
{
    Raw,
    Barebone,
    SmallestSize,
    BestPerformance,
    Balanced,
    BestSecurity,
}

impl Default
for ConfiguredFor
{
    fn default() -> ConfiguredFor {
        ConfiguredFor::Balanced
    }
}

pub struct ChainOfTrustBuilder
{
    pub(super) cfg: Config, 
    pub(super) configured_for: ConfiguredFor,
    pub(super) validators: Vec<Box<dyn EventValidator>>,
    pub(super) compactors: Vec<Box<dyn EventCompactor>>,
    pub(super) linters: Vec<Box<dyn EventMetadataLinter>>,
    pub(super) transformers: Vec<Box<dyn EventDataTransformer>>,
    pub(super) indexers: Vec<Box<dyn EventIndexer>>,
    pub(super) plugins: Vec<Box<dyn EventPlugin>>,
    pub(super) tree: Option<TreeAuthorityPlugin>,
    pub(super) truncate: bool,
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
            tree: self.tree.clone(),
            truncate: self.truncate,
        }
    }
}

impl ChainOfTrustBuilder
{
    #[allow(dead_code)]
    pub fn new(cfg: &Config) -> ChainOfTrustBuilder {
        ChainOfTrustBuilder {
            cfg: cfg.clone(),
            configured_for: cfg.configured_for.clone(),
            validators: Vec::new(),
            indexers: Vec::new(),
            compactors: Vec::new(),
            linters: Vec::new(),
            transformers: Vec::new(),
            plugins: Vec::new(),
            tree: None,
            truncate: false,
        }
        .with_defaults()
    }

    #[allow(dead_code)]
    pub fn with_defaults(mut self) -> Self {
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

        match self.configured_for {
            ConfiguredFor::SmallestSize | ConfiguredFor::Balanced => {
                self.transformers.insert(0, Box::new(CompressorWithSnapTransformer::default()));
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

            let tolerance = match self.configured_for {
                ConfiguredFor::BestPerformance => 2000,
                ConfiguredFor::BestSecurity => 200,
                _ => 500,
            };
            self.plugins.push(Box::new(TimestampEnforcer::new(&self.cfg, tolerance).unwrap()));
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
    pub fn add_root_public_key(mut self, key: &PublicKey) -> Self {
        if let Some(tree) = &mut self.tree {
            tree.add_root_public_key(key);
        }
        self
    }

    #[allow(dead_code)]
    pub async fn build
    (
        self,
        key: &ChainKey,
    )
    -> Result<ChainAccessor, ChainCreationError>
    {
        ChainAccessor::new(self, key).await
    }
}

impl Default
for ChainOfTrustBuilder
{
    fn default() -> ChainOfTrustBuilder {
        let cfg = Config::default();
        ChainOfTrustBuilder::new(&cfg)
    }
}

#[test]
fn test_config_mocking() {
    let cfg = mock_test_config();
    assert_eq!(cfg.roots.iter().next().unwrap().ip.to_string(), "127.0.0.1");
}