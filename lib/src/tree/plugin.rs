#[allow(unused_imports)]
use log::{error, info, warn, debug};
use fxhash::FxHashMap;
use std::sync::Arc;

use crate::crypto::*;
use crate::signature::*;
use crate::error::*;
use crate::sink::*;
use crate::meta::*;
use crate::plugin::*;
use crate::event::*;
use crate::header::*;
use crate::transaction::*;
use crate::trust::*;

#[derive(Debug, Clone)]
pub struct TreeAuthorityPlugin
{
    pub(super) root: WriteOption,
    pub(super) root_keys: FxHashMap<AteHash, PublicSignKey>,
    pub(super) auth: FxHashMap<PrimaryKey, MetaAuthorization>,
    pub(super) parents: FxHashMap<PrimaryKey, MetaParent>,
    pub(super) signature_plugin: SignaturePlugin,
    pub(super) integrity: IntegrityMode,
}

impl TreeAuthorityPlugin
{
    pub fn new() -> TreeAuthorityPlugin {
        TreeAuthorityPlugin {
            root: WriteOption::Everyone,
            root_keys: FxHashMap::default(),
            signature_plugin: SignaturePlugin::new(),
            auth: FxHashMap::default(),
            parents: FxHashMap::default(),
            integrity: IntegrityMode::Distributed,
        }
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(&mut self, key: &PublicSignKey)
    {
        self.root_keys.insert(key.hash(), key.clone());
        self.root = WriteOption::Any(self.root_keys.keys().map(|k| k.clone()).collect::<Vec<_>>());
    }
}

impl EventPlugin
for TreeAuthorityPlugin
{
    fn clone_plugin(&self) -> Box<dyn EventPlugin> {
        Box::new(self.clone())
    }

    fn rebuild(&mut self, headers: &Vec<EventHeader>, conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        self.reset();
        self.signature_plugin.rebuild(headers, conversation)?;
        for header in headers {
            match self.feed(header, conversation) {
                Ok(_) => { },
                Err(err) => {
                    debug!("feed error: {}", err);
                }
            }
        }
        Ok(())
    }

    fn root_keys(&self) -> Vec<PublicSignKey>
    {
        self.root_keys.values().map(|a| a.clone()).collect::<Vec<_>>()
    }

    fn set_root_keys(&mut self, root_keys: &Vec<PublicSignKey>)
    {
        self.root_keys.clear();
        self.root = WriteOption::Everyone;

        for root_key in root_keys {
            #[cfg(feature = "enable_verbose")]
            debug!("old_chain_root_key: {}", self.root);
            debug!("chain_root_key: {}", root_key.hash().to_string());
            self.add_root_public_key(root_key);
        }
    }
}