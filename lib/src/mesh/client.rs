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

pub(super) struct MeshClient {
    cfg_ate: ConfAte,
    lookup: MeshHashTable,
    sessions: Mutex<FxHashMap<ChainKey, Weak<MeshSession>>>,
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

    async fn open_internal<'a>(&'a self, mut key: ChainKey, ethereal: bool)
        -> Result<Arc<MeshSession>, ChainCreationError>
    {
        if key.to_string().starts_with("/") == false {
            key = ChainKey::from(format!("/{}", key.to_string()));
        }

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
        
        let builder = ChainOfTrustBuilder::new(&self.cfg_ate);
        let session = MeshSession::connect(builder, &key, addrs, ethereal).await?;
        *record = Arc::downgrade(&session);

        Ok(session)
    }
}

#[async_trait]
impl Mesh
for MeshClient {
    async fn open<'a>(&'a self, key: ChainKey)
        -> Result<Arc<MeshSession>, ChainCreationError>
    {
        self.persistent(key).await
    }

    async fn persistent<'a>(&'a self, key: ChainKey)
        -> Result<Arc<MeshSession>, ChainCreationError>
    {
        self.open_internal(key, false).await
    }

    async fn ethereal<'a>(&'a self, key: ChainKey)
        -> Result<Arc<MeshSession>, ChainCreationError>
    {
        self.open_internal(key, true).await
    }
}

impl Drop
for MeshClient
{
    fn drop(&mut self) {
        debug!("drop");
    }
}