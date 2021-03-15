use async_trait::async_trait;
use tokio::sync::{Mutex};
use std::{sync::Arc, collections::hash_map::Entry};
use fxhash::FxHashMap;
use crate::{header::PrimaryKey, pipe::EventPipe};
use std::sync::Weak;

use super::core::*;
use super::session::*;
use crate::accessor::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::msg::*;

pub(super) struct MeshClient {
    cfg: Config,
    lookup: MeshHashTable,
    sessions: Mutex<FxHashMap<ChainKey, Weak<MeshSession>>>,
}

impl MeshClient {
    pub(super) async fn new(cfg: &Config) -> Arc<MeshClient>
    {
        Arc::new(
            MeshClient
            {
                cfg: cfg.clone(),
                lookup: MeshHashTable::new(cfg),
                sessions: Mutex::new(FxHashMap::default()),
            }
        )
    }
}

#[async_trait]
impl Mesh
for MeshClient {
    async fn open<'a>(&'a self, key: ChainKey)
        -> Result<Arc<Chain>, ChainCreationError>
    {
        let mut sessions = self.sessions.lock().await;
        let record = match sessions.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Weak::new())
        };

        if let Some(ret) = record.upgrade() {
            return Ok(Arc::clone(&ret.chain));
        }

        let addrs = self.lookup.lookup(&key);
        if addrs.len() <= 0 {
            return Err(ChainCreationError::NoRootFound);
        }
        
        let builder = ChainOfTrustBuilder::new(&self.cfg);
        let session = MeshSession::new(builder, &key, addrs).await?;
        *record = Arc::downgrade(&session);

        Ok(Arc::clone(&session.chain))
    }
}