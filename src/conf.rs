use crate::{chain::ChainAccessorExt, time::EventTimestampLinter, tree::TreeAuthorityPlugin};
use std::sync::Arc;
use std::sync::RwLock;

use super::meta::OtherMetadata;
use super::validator::EventValidator;
use super::compact::EventCompactor;
use super::index::EventIndexer;
use super::lint::EventMetadataLinter;
use super::transform::EventDataTransformer;
use super::plugin::EventPlugin;
use super::compact::RemoveDuplicatesCompactor;
use super::compact::TombstoneCompactor;
use super::validator::RubberStampValidator;
use super::transform::CompressorWithSnapTransformer;
use super::chain::ChainKey;
use super::crypto::PublicKey;
use super::error::*;

pub trait ConfigMaster {
    fn master_addr(&self) -> String;
    fn master_port(&self) -> u32;
}

pub trait ConfigStorage {
    fn log_path(&self) -> String;
    fn log_temp(&self) -> bool;
}
pub trait Config: ConfigMaster + ConfigStorage {
}

#[derive(Default)]
pub struct DiscreteConfig {
    pub master_addr: String,
    pub master_port: u32,

    pub log_path: String,
    pub log_temp: bool,
}

impl DiscreteConfig {
    #[allow(dead_code)]
    pub fn with_master_addr(mut self, val: String) -> DiscreteConfig {
        self.master_addr = val;
        self
    }

    #[allow(dead_code)]
    pub fn with_master_port(mut self, val: u32) -> DiscreteConfig {
        self.master_port = val;
        self
    }

    #[allow(dead_code)]
    pub fn with_log_path(mut self, val: String) -> DiscreteConfig {
        self.log_path = val;
        self
    }

    #[allow(dead_code)]
    pub fn with_log_temp(mut self, val: bool) -> DiscreteConfig {
        self.log_temp = val;
        self
    }
}

impl ConfigMaster for DiscreteConfig {
    fn master_addr(&self) -> String { self.master_addr.clone() }
    fn master_port(&self) -> u32 { self.master_port }
}


impl ConfigStorage for DiscreteConfig {
    fn log_path(&self) -> String { self.log_path.clone() }
    fn log_temp(&self) -> bool { self.log_temp }
}


impl Config for DiscreteConfig {
}

#[cfg(test)]
pub fn mock_test_config() -> DiscreteConfig {
    DiscreteConfig::default()
        .with_master_addr("127.0.0.1".to_string())
        .with_master_port(4001)
        .with_log_path("/tmp/ate".to_string())
        .with_log_temp(true)
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



pub struct ChainOfTrustBuilderExt<M>
where M: OtherMetadata,
{
    pub(super) configured_for: ConfiguredFor,
    pub(super) validators: Vec<Box<dyn EventValidator<M>>>,
    pub(super) compactors: Vec<Box<dyn EventCompactor<M>>>,
    pub(super) linters: Vec<Box<dyn EventMetadataLinter<M>>>,
    pub(super) transformers: Vec<Box<dyn EventDataTransformer<M>>>,
    pub(super) indexers: Vec<Arc<RwLock<dyn EventIndexer<M>>>>,
    pub(super) plugins: Vec<Arc<RwLock<dyn EventPlugin<M>>>>,
    pub(super) tree: Option<Arc<RwLock<TreeAuthorityPlugin<M>>>>,
}

impl<M> ChainOfTrustBuilderExt<M>
where M: OtherMetadata + 'static,
{
    #[allow(dead_code)]
    pub fn new(flavour: ConfiguredFor) -> ChainOfTrustBuilderExt<M> {
        ChainOfTrustBuilderExt {
            configured_for: flavour.clone(),
            validators: Vec::new(),
            indexers: Vec::new(),
            compactors: Vec::new(),
            linters: Vec::new(),
            transformers: Vec::new(),
            plugins: Vec::new(),
            tree: None,
        }
        .with_defaults(flavour)
    }

    #[allow(dead_code)]
    pub fn with_defaults(mut self, flavour: ConfiguredFor) -> Self {
        self.validators.clear();
        self.indexers.clear();
        self.linters.clear();
        self.transformers.clear();
        self.plugins.clear();
        self.compactors.clear();
        self.tree = None;

        if flavour == ConfiguredFor::Raw {
            return self;
        }

        self.compactors.push(Box::new(RemoveDuplicatesCompactor::default()));
        self.compactors.push(Box::new(TombstoneCompactor::default()));
        self.linters.push(Box::new(EventTimestampLinter::default()));

        if flavour == ConfiguredFor::Barebone {
            self.validators.push(Box::new(RubberStampValidator::default()));
            return self;
        }
        else
        {
            let tree = Arc::new(RwLock::new(super::tree::TreeAuthorityPlugin::new()));
            self.tree = Some(tree.clone());
            self.plugins.push(tree);
        }

        match flavour {
            ConfiguredFor::SmallestSize | ConfiguredFor::Balanced => {
                self.transformers.insert(0, Box::new(CompressorWithSnapTransformer::default()));
            }
            _ => {}
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
        self
    }

    #[allow(dead_code)]
    pub fn add_compactor(mut self, compactor: Box<dyn EventCompactor<M>>) -> Self {
        self.compactors.push(compactor);
        self
    }

    #[allow(dead_code)]
    pub fn add_validator(mut self, validator: Box<dyn EventValidator<M>>) -> Self {
        self.validators.push(validator);
        self
    }
    #[allow(dead_code)]
    pub fn add_metadata_linter(mut self, linter: Box<dyn EventMetadataLinter<M>>) -> Self {
        self.linters.push(linter);
        self
    }

    #[allow(dead_code)]
    pub fn add_data_transformer(mut self, transformer: Box<dyn EventDataTransformer<M>>) -> Self {
        self.transformers.push(transformer);
        self
    }

    #[allow(dead_code)]
    pub fn add_indexer(mut self, indexer: &Arc<RwLock<dyn EventIndexer<M>>>) -> Self {
        self.indexers.push(indexer.clone());
        self
    }


    #[allow(dead_code)]
    pub fn add_plugin(mut self, plugin: &Arc<RwLock<dyn EventPlugin<M>>>) -> Self {
        self.plugins.push(plugin.clone());
        self
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(self, key: &PublicKey) -> Self {
        if let Some(tree) = &self.tree {
            tree.write().unwrap().add_root_public_key(key);
        }
        self
    }

    #[allow(dead_code)]
    pub fn build<I, V>
    (
        self,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool
    ) -> Result<ChainAccessorExt<M>, ChainCreationError>
    {
        ChainAccessorExt::new(self, cfg, key, truncate)
    }
}

impl<M> Default
for ChainOfTrustBuilderExt<M>
where M: OtherMetadata + 'static,
{
    fn default() -> ChainOfTrustBuilderExt<M> {
        ChainOfTrustBuilderExt::new(ConfiguredFor::default())
    }
}

#[test]
fn test_config_mocking() {
    let cfg = mock_test_config();
    assert_eq!(cfg.master_addr(), "127.0.0.1");
}