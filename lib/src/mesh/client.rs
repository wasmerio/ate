use crate::{header::PrimaryKey, pipe::EventPipe};
use async_trait::async_trait;
use error_chain::bail;
use fxhash::FxHashMap;
use std::sync::Weak;
use std::time::Duration;
use std::{collections::hash_map::Entry, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_futures::{Instrument, WithSubscriber};

use super::core::*;
use super::msg::*;
use super::session::*;
use crate::chain::*;
use crate::comms::StreamProtocol;
use crate::conf::*;
use crate::error::*;
use crate::loader::Loader;
use crate::prelude::NodeId;
use crate::prelude::TaskEngine;
use crate::transaction::*;
use crate::trust::*;

pub struct MeshClient {
    cfg_ate: ConfAte,
    cfg_mesh: ConfMesh,
    lookup: MeshHashTable,
    node_id: NodeId,
    temporal: bool,
    sessions: Mutex<FxHashMap<ChainKey, Arc<MeshClientSession>>>,
}

pub struct MeshClientSession {
    key: ChainKey,
    chain: Mutex<Weak<Chain>>,
}

impl MeshClientSession {
    pub(crate) async fn __try_open_ext<'a>(
        &'a self,
    ) -> Result<Option<Arc<Chain>>, ChainCreationError> {
        let chain = self.chain.lock().await;
        if let Some(chain) = chain.upgrade() {
            trace!("reusing chain {}", self.key);
            return Ok(Some(chain));
        }
        Ok(None)
    }

    pub(crate) async fn __open_ext<'a>(
        &'a self,
        client: &MeshClient,
        hello_path: String,
        loader_local: impl Loader + 'static,
        loader_remote: impl Loader + 'static,
    ) -> Result<Arc<Chain>, ChainCreationError> {
        let mut chain = self.chain.lock().await;
        if let Some(chain) = chain.upgrade() {
            trace!("reusing chain {}", self.key);
            return Ok(chain);
        }

        trace!("creating chain {}", self.key);
        let ret = self
            .__open_ext_internal(client, hello_path, loader_local, loader_remote)
            .await?;
        *chain = Arc::downgrade(&ret);
        Ok(ret)
    }

    async fn __open_ext_internal<'a>(
        &'a self,
        client: &MeshClient,
        hello_path: String,
        loader_local: impl Loader + 'static,
        loader_remote: impl Loader + 'static,
    ) -> Result<Arc<Chain>, ChainCreationError> {
        debug!(key = self.key.to_string().as_str());
        debug!(path = hello_path.as_str());

        let (peer_addr, _) = match client.lookup.lookup(&self.key) {
            Some(a) => a,
            None => {
                bail!(ChainCreationErrorKind::NoRootFoundInConfig);
            }
        };
        let addr = match &client.cfg_mesh.force_connect {
            Some(a) => a.clone(),
            None => peer_addr,
        };

        let builder = ChainBuilder::new(&client.cfg_ate)
            .await
            .node_id(client.node_id.clone())
            .temporal(client.temporal);

        trace!("connecting to {}", addr);
        let chain = MeshSession::connect(
            builder,
            &client.cfg_mesh,
            &self.key,
            addr,
            client.node_id.clone(),
            hello_path,
            loader_local,
            loader_remote,
        )
        .await?;

        Ok(chain)
    }
}

impl MeshClient {
    pub(super) fn new(
        cfg_ate: &ConfAte,
        cfg_mesh: &ConfMesh,
        node_id: NodeId,
        temporal: bool,
    ) -> Arc<MeshClient> {
        Arc::new(MeshClient {
            cfg_ate: cfg_ate.clone(),
            cfg_mesh: cfg_mesh.clone(),
            lookup: MeshHashTable::new(cfg_mesh),
            node_id,
            temporal,
            sessions: Mutex::new(FxHashMap::default()),
        })
    }

    pub async fn open_ext<'a>(
        &'a self,
        key: &ChainKey,
        hello_path: String,
        loader_local: impl Loader + 'static,
        loader_remote: impl Loader + 'static,
    ) -> Result<Arc<Chain>, ChainCreationError> {
        let span = span!(
            Level::INFO,
            "client-open",
            id = self.node_id.to_short_string().as_str()
        );
        TaskEngine::run_until(
            self.__open_ext(key, hello_path, loader_local, loader_remote)
                .instrument(span),
        )
        .await
    }

    pub(crate) async fn __try_open_ext<'a>(
        &'a self,
        key: &ChainKey,
    ) -> Result<Option<Arc<Chain>>, ChainCreationError> {
        let session = {
            let mut sessions = self.sessions.lock().await;
            let record = match sessions.entry(key.clone()) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(_) => {
                    return Ok(None);
                }
            };
            Arc::clone(record)
        };

        session.__try_open_ext().await
    }

    pub(crate) async fn __open_ext<'a>(
        &'a self,
        key: &ChainKey,
        hello_path: String,
        loader_local: impl Loader + 'static,
        loader_remote: impl Loader + 'static,
    ) -> Result<Arc<Chain>, ChainCreationError> {
        let session = {
            let mut sessions = self.sessions.lock().await;
            let record = match sessions.entry(key.clone()) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(v) => v.insert(Arc::new(MeshClientSession {
                    key: key.clone(),
                    chain: Mutex::new(Weak::new()),
                })),
            };
            Arc::clone(record)
        };

        session
            .__open_ext(self, hello_path, loader_local, loader_remote)
            .await
    }

    pub fn temporal(mut self, val: bool) -> Self {
        self.temporal = val;
        self
    }
}

impl Drop for MeshClient {
    fn drop(&mut self) {
        let span = span!(
            Level::TRACE,
            "client",
            id = self.node_id.to_short_string().as_str()
        );
        let _span = span.enter();

        trace!("drop (out-of-scope)");
    }
}

impl MeshClient {
    pub async fn open(
        self: &Arc<MeshClient>,
        url: &'_ url::Url,
        key: &'_ ChainKey,
    ) -> Result<Arc<Chain>, ChainCreationError> {
        TaskEngine::run_until(self.__open(url, key)).await
    }

    async fn __open(
        self: &Arc<MeshClient>,
        url: &'_ url::Url,
        key: &'_ ChainKey,
    ) -> Result<Arc<Chain>, ChainCreationError> {
        let loader_local = crate::loader::DummyLoader::default();
        let loader_remote = crate::loader::DummyLoader::default();
        let hello_path = url.path().to_string();
        self.__open_ext(&key, hello_path, loader_local, loader_remote)
            .await
    }
}
