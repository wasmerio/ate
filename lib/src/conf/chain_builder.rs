#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Arc;
use parking_lot::Mutex as StdMutex;

use crate::anti_replay::AntiReplayPlugin;
use crate::chain::Chain;
use crate::time::TimestampEnforcer;
use crate::tree::TreeAuthorityPlugin;
use crate::validator::*;
use crate::compact::*;
use crate::index::*;
use crate::lint::*;
use crate::transform::*;
use crate::plugin::*;
use crate::trust::ChainKey;
use crate::crypto::PublicSignKey;
use crate::error::*;
use crate::pipe::*;
use crate::engine::*;
use crate::session::AteSessionUser;
use crate::session::AteSession;
use crate::comms::NodeId;
use crate::comms::Metrics;
use crate::comms::Throttle;
use crate::prelude::TrustMode;
use crate::prelude::CentralizedRole;

use super::*;

/// Building class used to construct a chain-of-trust with
/// its user defined plugins and configuration. Nearly always
/// this builder will be used to create and load your chains.
pub struct ChainBuilder
{
    pub(crate) cfg_ate: ConfAte,
    pub(crate) node_id: NodeId,
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
    pub(crate) session: Box<dyn AteSession>,
    pub(crate) metrics: Arc<StdMutex<Metrics>>,
    pub(crate) throttle: Arc<StdMutex<Throttle>>,
    pub(crate) load_integrity: TrustMode,
    pub(crate) idle_integrity: TrustMode,
}

impl Clone
for ChainBuilder
{
    fn clone(&self) -> Self {
        ChainBuilder {
            cfg_ate: self.cfg_ate.clone(),
            node_id: self.node_id.clone(),
            configured_for: self.configured_for.clone(),
            validators: self.validators.iter().map(|a| a.clone_validator()).collect::<Vec<_>>(),
            compactors: self.compactors.iter().filter_map(|a| a.clone_compactor()).collect::<Vec<_>>(),
            linters: self.linters.iter().map(|a| a.clone_linter()).collect::<Vec<_>>(),
            transformers: self.transformers.iter().map(|a| a.clone_transformer()).collect::<Vec<_>>(),
            indexers: self.indexers.iter().map(|a| a.clone_indexer()).collect::<Vec<_>>(),
            plugins: self.plugins.iter().map(|a| a.clone_plugin()).collect::<Vec<_>>(),
            pipes: self.pipes.clone(),
            tree: self.tree.clone(),
            session: self.session.clone_session(),
            truncate: self.truncate,
            temporal: self.temporal,
            metrics: Arc::clone(&self.metrics),
            throttle: Arc::clone(&self.throttle),
            load_integrity: self.load_integrity,
            idle_integrity: self.idle_integrity,
        }
    }
}

impl ChainBuilder
{
    #[allow(dead_code)]
    pub async fn new(cfg_ate: &ConfAte) -> ChainBuilder {
        ChainBuilder {
            cfg_ate: cfg_ate.clone(),
            node_id: crate::comms::NodeId::generate_client_id(),
            configured_for: cfg_ate.configured_for.clone(),
            validators: Vec::new(),
            indexers: Vec::new(),
            compactors: Vec::new(),
            linters: Vec::new(),
            transformers: Vec::new(),
            plugins: Vec::new(),
            pipes: None,
            tree: None,
            session: AteSessionUser::new().into(),
            truncate: false,
            temporal: false,
            metrics: Arc::new(StdMutex::new(Metrics::default())),
            throttle: Arc::new(StdMutex::new(Throttle::default())),
            load_integrity: TrustMode::Centralized(CentralizedRole::Client),
            idle_integrity: TrustMode::Distributed
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

        self.compactors.push(Box::new(SignatureCompactor::default()));
        self.compactors.push(Box::new(RemoveDuplicatesCompactor::default()));
        self.compactors.push(Box::new(TombstoneCompactor::default()));
        self.plugins.push(Box::new(AntiReplayPlugin::default()));

        match self.configured_for {
            ConfiguredFor::SmallestSize => {
                self.transformers.insert(0, Box::new(CompressorWithSnapTransformer::default()));
            },
            ConfiguredFor::Balanced => {
            },
            ConfiguredFor::BestSecurity => {
                self.cfg_ate.dns_sec = true;
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

            let tolerance = self.configured_for.ntp_tolerance();
            self.plugins.push(Box::new(TimestampEnforcer::new(&self.cfg_ate, tolerance).await.unwrap()));
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
    pub fn with_metrics(mut self, metrics: &Arc<StdMutex<Metrics>>) -> Self {
        self.metrics = Arc::clone(metrics);
        self
    }

    #[allow(dead_code)]
    pub fn with_throttle(mut self, throttle: &Arc<StdMutex<Throttle>>) -> Self {
        self.throttle = Arc::clone(throttle);
        self
    }

    #[allow(dead_code)]
    pub fn load_integrity(mut self, trust: TrustMode) -> Self {
        self.load_integrity = trust;
        self
    }

    #[allow(dead_code)]
    pub fn idle_integrity(mut self, trust: TrustMode) -> Self {
        self.idle_integrity = trust;
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

    #[cfg(feature = "enable_local_fs")]
    #[allow(dead_code)]
    pub fn postfix_log_path(mut self, postfix: &str) -> Self
    {
        let orig_path = match self.cfg_ate.log_path.as_ref() {
            Some(a) => a.clone(),
            None => {
                return self;
            }
        };

        // Remove any prefix slash as this will already be there
        let mut postfix = postfix.to_string();
        while postfix.starts_with("/") {
            postfix = postfix[1..].to_string();
        }
        if postfix.len() <= 0 {
            return self;
        }

        let path = match orig_path.ends_with("/") {
            true => format!("{}{}", orig_path, postfix),
            false => format!("{}/{}", orig_path, postfix)
        };
        self.cfg_ate.log_path = Some(path);

        // Also update the backup path
        if let Some(backup_path) = self.cfg_ate.backup_path.as_ref() {
            let backup_path = backup_path.clone();

            let path = match backup_path.ends_with("/") {
                true => format!("{}{}", backup_path, postfix),
                false => format!("{}/{}", backup_path, postfix)
            };
            self.cfg_ate.backup_path = Some(path);
        }

        self
    }

    #[allow(dead_code)]
    pub fn set_session(mut self, session: Box<dyn AteSession>) -> Self {
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

    pub fn node_id(mut self, client_id: NodeId) -> Self {
        self.node_id = client_id;
        self
    }

    pub fn cfg_ate(&self) -> &ConfAte {
        &self.cfg_ate
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

    pub async fn open(self: &Arc<Self>,
                      key: &ChainKey
    ) -> Result<Arc<Chain>, ChainCreationError>
    {
        TaskEngine::run_until(self.__open(key)).await
    }
    
    async fn __open(self: &Arc<Self>,
                    key: &ChainKey
    ) -> Result<Arc<Chain>, ChainCreationError>
    {
        let ret = Arc::new(Chain::new((**self).clone(), key, self.load_integrity, self.idle_integrity).await?);
        Ok(ret)
    }
}