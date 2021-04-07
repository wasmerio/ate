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
    sessions: Mutex<FxHashMap<ChainKey, Weak<Chain>>>,
}

impl MeshClient {
    pub(super) async fn new(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient>
    {
        Arc::new(
            MeshClient
            {
                cfg_ate: cfg_ate.clone(),
                lookup: MeshHashTable::new(cfg_mesh),
                sessions: Mutex::new(FxHashMap::default()),
            }
        )
    }

    async fn open_internal<'a>(&'a self, url: &url::Url, loader_local: Box<impl Loader>, loader_remote: Box<impl Loader>)
        -> Result<Arc<Chain>, ChainCreationError>
    {
        let key = ChainKey::from_url(url);
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
        
        let builder = ChainOfTrustBuilder::new(&self.cfg_ate).await;
        let (_, chain) = MeshSession::connect(builder, url, addrs, loader_local, loader_remote).await?;
        *record = Arc::downgrade(&chain);

        Ok(chain)
    }

    pub async fn open_ext(&self, url: &url::Url, loader_local: Box<impl Loader>, loader_remote: Box<impl Loader>)
        -> Result<Arc<Chain>, ChainCreationError>
    {
        self.open_internal(url, loader_local, loader_remote).await
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
    async fn open(&self, url: &url::Url) -> Result<Arc<Chain>, ChainCreationError>
    {
        let loader_local  = Box::new(crate::loader::DummyLoader::default());
        let loader_remote  = Box::new(crate::loader::DummyLoader::default());
        self.open_internal(url, loader_local, loader_remote).await
    }
}