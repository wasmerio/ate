use ate::prelude::*;
use ate_auth::error::GatherError;
use bytes::Bytes;
use std::ops::Deref;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use ttl_cache::TtlCache;

use crate::prelude::*;

pub struct Repository {
    pub registry: Arc<Registry>,
    pub db_url: url::Url,
    pub auth_url: url::Url,
    pub sessions: RwLock<TtlCache<String, AteSessionType>>,
    pub chains: Mutex<TtlCache<ChainKey, Arc<FileAccessor>>>,
    pub session_factory: Box<dyn Fn(&str, ChainKey) -> Pin<Box<dyn Future<Output=Result<AteSessionType, AteError>> + Send>> + Send + Sync + 'static>,
    pub ttl: Duration,
}

impl Repository {
    pub async fn new<Fut>(
        registry: &Arc<Registry>,
        db_url: url::Url,
        auth_url: url::Url,
        session_factory: impl Fn(&str, ChainKey) -> Fut + Send + Sync + 'static,
        ttl: Duration,
    ) -> Result<Arc<Repository>, AteError>
    where Fut: Future<Output=Result<AteSessionType, AteError>>,
          Fut: Send + 'static,
    {
        let ret = Repository {
            registry: Arc::clone(registry),
            db_url,
            auth_url,
            session_factory: Box::new(move |sni, key| {
                Box::pin(session_factory(sni, key))
            }),
            sessions: RwLock::new(TtlCache::new(usize::MAX)),
            chains: Mutex::new(TtlCache::new(usize::MAX)),
            ttl,
        };
        let ret = Arc::new(ret);
        Ok(ret)
    }
}

impl Repository {
    pub async fn get_session(&self, sni: &String, key: ChainKey) -> Result<AteSessionType, GatherError> {
        // Check the check
        {
            let guard = self.sessions.read().unwrap();
            if let Some(ret) = guard.get(sni) {
                return Ok(ret.clone());
            }
        }

        // Create the session
        let session_factory = self.session_factory.deref();
        let session = session_factory(sni.as_str(), key).await?;

        // Enter a write lock and check again
        let mut guard = self.sessions.write().unwrap();
        if let Some(ret) = guard.get(sni) {
            return Ok(ret.clone());
        }

        // Cache and and return it
        guard.insert(sni.clone(), session.clone(), self.ttl);
        Ok(session)
    }

    pub async fn get_accessor(&self, key: &ChainKey, sni: &str) -> Result<Arc<FileAccessor>, GatherError> {
        // Get the session
        let sni = sni.to_string();
        let session = self.get_session(&sni, key.clone()).await?;

        // Now get the chain for this host
        let chain = {
            let mut chains = self.chains.lock().await;
            if let Some(ret) = chains.remove(key) {
                chains.insert(key.clone(), Arc::clone(&ret), self.ttl);
                ret
            } else {
                let chain = self.registry.open(&self.db_url, &key).await?;
                let accessor = Arc::new(
                    FileAccessor::new(
                        chain.as_arc(),
                        Some(sni),
                        session,
                        TransactionScope::Local,
                        TransactionScope::Local,
                        false,
                        false,
                    )
                    .await,
                );
                chains.insert(key.clone(), Arc::clone(&accessor), self.ttl);
                accessor
            }
        };

        Ok(chain)
    }

    pub async fn get_file(&self, key: &ChainKey, sni: &str, path: &str) -> Result<Option<Bytes>, FileSystemError> {
        let path = path.to_string();
        let context = RequestContext::default();

        let chain = self.get_accessor(key, sni).await?;
        Ok(match chain.search(&context, path.as_str()).await {
            Ok(Some(a)) => {
                let flags = crate::codes::O_RDONLY as u32;
                let oh = match chain.open(&context, a.ino, flags).await {
                    Ok(a) => Some(a),
                    Err(FileSystemError(FileSystemErrorKind::IsDirectory, _)) => None,
                    Err(err) => {
                        return Err(err.into());
                    }
                };
                match oh {
                    Some(oh) => Some(chain.read(&context, a.ino, oh.fh, 0, u32::MAX).await?),
                    None => None,
                }
            }
            Ok(None) => None,
            Err(FileSystemError(FileSystemErrorKind::IsDirectory, _))
            | Err(FileSystemError(FileSystemErrorKind::DoesNotExist, _))
            | Err(FileSystemError(FileSystemErrorKind::NoEntry, _)) => None,
            Err(err) => {
                return Err(err.into());
            }
        })
    }

    pub async fn set_file(
        &self,
        key: &ChainKey,
        sni: &str,
        path: &str,
        data: &[u8],
    ) -> Result<u64, FileSystemError> {
        let path = path.to_string();
        let context = RequestContext::default();

        let chain = self.get_accessor(key, sni).await?;
        let file = chain.touch(&context, path.as_str()).await?;
        let flags = (crate::codes::O_RDWR as u32) | (crate::codes::O_TRUNC as u32);
        let oh = chain.open(&context, file.ino, flags).await?;
        chain
            .fallocate(&context, file.ino, oh.fh, 0, 0, flags)
            .await?;
        let written = chain
            .write(&context, file.ino, oh.fh, 0, data, flags)
            .await?;
        chain.sync(&context, file.ino, oh.fh, 0).await?;
        Ok(written)
    }

    pub async fn house_keeping(&self) {
        let mut lock = self.chains.lock().await;
        lock.iter(); // this will run the remove_expired function
    }
}
