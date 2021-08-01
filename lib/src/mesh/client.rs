use async_trait::async_trait;
use tokio::sync::{Mutex};
use tracing::{info, warn, debug, error, trace};
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
use crate::comms::StreamProtocol;
use crate::prelude::TaskEngine;

pub struct MeshClient {
    cfg_ate: ConfAte,
    cfg_mesh: ConfMesh,
    lookup: MeshHashTable,
    temporal: bool,
    sessions: Mutex<FxHashMap<ChainKey, Weak<Chain>>>,
}

impl MeshClient {
    pub(super) fn new(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, temporal: bool) -> Arc<MeshClient>
    {
        Arc::new(
            MeshClient
            {
                cfg_ate: cfg_ate.clone(),
                cfg_mesh: cfg_mesh.clone(),
                lookup: MeshHashTable::new(cfg_mesh),
                temporal,
                sessions: Mutex::new(FxHashMap::default()),
            }
        )
    }

    pub async fn open_ext<'a>(&'a self, key: &ChainKey, hello_path: String, loader_local: impl Loader + 'static, loader_remote: impl Loader + 'static)
        -> Result<Arc<Chain>, ChainCreationError>
    {
        debug!("client open {}", key.to_string());

        let mut sessions = self.sessions.lock().await;
        let record = match sessions.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Weak::new())
        };

        if let Some(ret) = record.upgrade() {
            return Ok(Arc::clone(&ret));
        }

        let addr = match &self.cfg_mesh.force_connect {
            Some(a) => a.clone(),
            None => {
                let addr = self.lookup.lookup(&key);
                match addr {
                    Some((a, _)) => a,
                    None => { return Err(ChainCreationError::NoRootFoundInConfig); }
                }
            }
        };
        
        let builder = ChainBuilder::new(&self.cfg_ate).await
            .temporal(self.temporal);

        let chain = MeshSession::connect
            (
                builder,
                &self.cfg_mesh,
                key,
                addr,
                hello_path,
                loader_local,
                loader_remote
            )
            .await?;
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
        trace!("drop");
    }
}

#[async_trait]
impl ChainRepository
for MeshClient
{
    async fn open(self: Arc<MeshClient>, url: &'_ url::Url, key: &'_ ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        TaskEngine::run_until(self.__open(url, key)).await
    }
}

impl MeshClient
{
    async fn __open(self: &Arc<MeshClient>, url: &'_ url::Url, key: &'_ ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let weak = Arc::downgrade(self);
        let loader_local  = crate::loader::DummyLoader::default();
        let loader_remote  = crate::loader::DummyLoader::default();
        let hello_path = url.path().to_string();
        let ret = self.open_ext(&key, hello_path, loader_local, loader_remote).await?;
        ret.inside_sync.write().repository = Some(weak);
        Ok(ret)
    }
}