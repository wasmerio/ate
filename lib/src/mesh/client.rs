use async_trait::async_trait;
use tokio::sync::{Mutex};
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::{Instrument, WithSubscriber};
use error_chain::bail;
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
use crate::comms::StreamProtocol;
use crate::prelude::TaskEngine;
use crate::prelude::NodeId;

pub struct MeshClient {
    cfg_ate: ConfAte,
    cfg_mesh: ConfMesh,
    lookup: MeshHashTable,
    client_id: NodeId,
    temporal: bool,
    sessions: Mutex<FxHashMap<ChainKey, Weak<Chain>>>,
}

impl MeshClient {
    pub(super) fn new(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, client_id: NodeId, temporal: bool) -> Arc<MeshClient>
    {
        Arc::new(
            MeshClient
            {
                cfg_ate: cfg_ate.clone(),
                cfg_mesh: cfg_mesh.clone(),
                lookup: MeshHashTable::new(cfg_mesh),
                client_id,
                temporal,
                sessions: Mutex::new(FxHashMap::default()),
            }
        )
    }

    pub async fn open_ext<'a>(&'a self, key: &ChainKey, hello_path: String, loader_local: impl Loader + 'static, loader_remote: impl Loader + 'static)
        -> Result<Arc<Chain>, ChainCreationError>
    {
        let span = span!(Level::INFO, "client-open", id=self.client_id.to_short_string().as_str());
        TaskEngine::run_until(self.__open_ext(key, hello_path, loader_local, loader_remote)
            .instrument(span)
        ).await
    }

    async fn __open_ext<'a>(&'a self, key: &ChainKey, hello_path: String, loader_local: impl Loader + 'static, loader_remote: impl Loader + 'static)
        -> Result<Arc<Chain>, ChainCreationError>
    {
        debug!(key=key.to_string().as_str());
        debug!(path=hello_path.as_str());

        let mut sessions = self.sessions.lock().await;
        let record = match sessions.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Weak::new())
        };

        if let Some(ret) = record.upgrade() {
            return Ok(Arc::clone(&ret));
        }

        let (peer_addr, _) = match self.lookup.lookup(&key) {
            Some(a) => a,
            None => { bail!(ChainCreationErrorKind::NoRootFoundInConfig); }
        };
        let addr = match &self.cfg_mesh.force_connect {
            Some(a) => a.clone(),
            None => peer_addr
        };
        
        let builder = ChainBuilder::new(&self.cfg_ate).await
            .client_id(self.client_id.clone())
            .temporal(self.temporal);

        let chain = MeshSession::connect
            (
                builder,
                &self.cfg_mesh,
                key,
                addr,
                self.client_id.clone(),
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
        
        let span = span!(Level::TRACE, "client", id=self.client_id.to_short_string().as_str());
        let _span = span.enter();

        trace!("drop (out-of-scope)");
    }
}

impl MeshClient
{
    pub async fn open(self: &Arc<MeshClient>, url: &'_ url::Url, key: &'_ ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        TaskEngine::run_until(self.__open(url, key)).await
    }

    async fn __open(self: &Arc<MeshClient>, url: &'_ url::Url, key: &'_ ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let loader_local  = crate::loader::DummyLoader::default();
        let loader_remote  = crate::loader::DummyLoader::default();
        let hello_path = url.path().to_string();
        self.open_ext(&key, hello_path, loader_local, loader_remote).await
    }
}