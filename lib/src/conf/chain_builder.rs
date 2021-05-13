#[allow(unused_imports)]
use log::{info, error, debug};
use async_trait::async_trait;
use std::sync::Arc;
use url::Url;

use crate::anti_replay::AntiReplayPlugin;
use crate::chain::Chain;
use crate::time::TimestampEnforcer;
use crate::tree::TreeAuthorityPlugin;
use crate::tree::TreeCompactor;
use crate::validator::*;
use crate::compact::*;
use crate::index::*;
use crate::lint::*;
use crate::transform::*;
use crate::plugin::*;
use crate::trust::ChainKey;
use crate::trust::IntegrityMode;
use crate::crypto::PublicSignKey;
use crate::crypto::KeySize;
use crate::error::*;
use crate::pipe::*;
use crate::session::AteSession;
use crate::repository::ChainRepository;

use super::*;

/// Building class used to construct a chain-of-trust with
/// its user defined plugins and configuration. Nearly always
/// this builder will be used to create and load your chains.
pub struct ChainBuilder
{
    pub(crate) cfg: ConfAte, 
    pub(crate) configured_for: ConfiguredFor,
    pub(crate) validators: Vec<Box<dyn EventValidator>>,
    pub(crate) compactors: Vec<Box<dyn EventCompactor>>,
    pub(crate) linters: Vec<Box<dyn EventMetadataLinter>>,
    pub(crate) transformers: Vec<Box<dyn EventDataTransformer>>,
    pub(crate) indexers: Vec<Box<dyn EventIndexer>>,
    pub(crate) plugins: Vec<Box<dyn EventPlugin>>,
    pub(crate) pipes: Option<Arc<Box<dyn EventPipe>>>,
    pub(crate) tree: Option<TreeAuthorityPlugin>,
    pub(crate) truncate: bool,
    pub(crate) temporal: bool,
    pub(crate) integrity: IntegrityMode,
    pub(crate) session: AteSession,
}

impl Clone
for ChainBuilder
{
    fn clone(&self) -> Self {
        ChainBuilder {
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

impl ChainBuilder
{
    #[allow(dead_code)]
    pub async fn new(cfg: &ConfAte) -> ChainBuilder {
        ChainBuilder {
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
            session: AteSession::new(&cfg),
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

        self.compactors.push(Box::new(KeepDataCompactor::default()));
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
            self.tree = Some(crate::tree::TreeAuthorityPlugin::new());
            self.compactors.push(Box::new(TreeCompactor::default()));

            let tolerance = self.configured_for.ntp_tolerance();
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
    pub fn set_session(mut self, session: AteSession) -> Self {
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
    -> Arc<ChainBuilder>
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
for ChainBuilder
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