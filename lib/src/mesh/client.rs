use async_trait::async_trait;
use tokio::sync::{Mutex};
use log::{warn, debug, error};
use std::{sync::Arc, collections::hash_map::Entry};
use fxhash::FxHashMap;
use crate::{header::PrimaryKey, pipe::EventPipe};
use std::sync::Weak;

use super::core::*;
use super::session::*;
use crate::trust::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::msg::*;
use crate::loader::Loader;
use crate::repository::ChainRepository;

pub struct MeshClient {
    cfg_ate: ConfAte,
    lookup: MeshHashTable,
    temporal: bool,
    sessions: Mutex<FxHashMap<ChainKey, Weak<Chain>>>,
}

impl MeshClient {
    pub(super) async fn new(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, temporal: bool) -> Arc<MeshClient>
    {
        Arc::new(
            MeshClient
            {
                cfg_ate: cfg_ate.clone(),
                lookup: MeshHashTable::new(cfg_mesh),
                temporal,
                sessions: Mutex::new(FxHashMap::default()),
            }
        )
    }

    pub async fn open_ext<'a>(&'a self, key: &ChainKey, domain: Option<String>, loader_local: Box<impl Loader>, loader_remote: Box<impl Loader>)
        -> Result<Arc<Chain>, ChainCreationError>
    {
        debug!("open {}", key.to_string());

        let mut sessions = self.sessions.lock().await;
        let record = match sessions.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Weak::new())
        };

        if let Some(ret) = record.upgrade() {
            return Ok(Arc::clone(&ret));
        }

        let addrs = self.lookup.lookup(&key);
        if addrs.len() <= 0 {
            return Err(ChainCreationError::NoRootFoundInConfig);
        }
        
        let builder = ChainOfTrustBuilder::new(&self.cfg_ate).await
            .temporal(self.temporal);
        let (_, chain) = MeshSession::connect(builder, key, domain, addrs, loader_local, loader_remote).await?;
        *record = Arc::downgrade(&chain);

        Ok(chain)
    }

    pub fn temporal(mut self, val: bool) -> Self
    {
        self.temporal = val;
        self
    }
}

impl Drop
for MeshClient
{
    fn drop(&mut self) {
        debug!("drop");
    }
}

#[async_trait]
impl ChainRepository
for MeshClient
{
    async fn open_by_url(self: Arc<Self>, url: &url::Url) -> Result<Arc<Chain>, ChainCreationError>
    {
        let domain = url.domain().map(|a| a.to_string());
        let key = ChainKey::from_url(url);
        let weak = Arc::downgrade(&self);
        let loader_local  = Box::new(crate::loader::DummyLoader::default());
        let loader_remote  = Box::new(crate::loader::DummyLoader::default());
        let ret = self.open_ext(&key, domain, loader_local, loader_remote).await?;
        ret.inside_sync.write().repository = Some(weak);
        Ok(ret)
    }

    async fn open_by_key(self: Arc<Self>, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let weak = Arc::downgrade(&self);
        let loader_local  = Box::new(crate::loader::DummyLoader::default());
        let loader_remote  = Box::new(crate::loader::DummyLoader::default());
        let ret = self.open_ext(key, None, loader_local, loader_remote).await?;
        ret.inside_sync.write().repository = Some(weak);
        Ok(ret)
    }
}