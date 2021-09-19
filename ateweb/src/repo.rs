#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Arc;
use ate::prelude::*;
use parking_lot::RwLock;
use tokio::sync::Mutex;
use std::time::Duration;
use ate_auth::service::AuthService;
use ate_auth::cmd::gather_command;
use ate_auth::error::GatherError;
use ate_files::prelude::*;
use ttl_cache::TtlCache;
use bytes::Bytes;

pub struct Repository
{
    pub registry: Arc<Registry>,
    pub db_url: url::Url,
    pub auth_url: url::Url,
    pub web_key: EncryptKey,
    pub sessions: RwLock<TtlCache<String, AteSessionGroup>>,
    pub chains: Mutex<TtlCache<String, Arc<FileAccessor>>>,
    pub ttl: Duration,
}

impl Repository
{
    pub async fn new(registry: &Arc<Registry>, db_url: url::Url, auth_url: url::Url, web_key: EncryptKey, ttl: Duration) -> Result<Arc<Repository>, AteError>
    {
        let ret = Repository {
            registry: Arc::clone(registry),
            db_url,
            auth_url,
            web_key,
            sessions: RwLock::new(TtlCache::new(usize::MAX)),
            chains: Mutex::new(TtlCache::new(usize::MAX)),
            ttl,
        };
        let ret = Arc::new(ret);
        Ok(ret)
    }
}

impl Repository
{
    pub async fn get_session(&self, sni: &String) -> Result<AteSessionGroup, GatherError>
    {
        // Check the check
        {
            let guard = self.sessions.read();
            if let Some(ret) = guard.get(sni) {
                return Ok(ret.clone());
            }
        }

        // Create the session
        let web_key_entropy = format!("web-read:{}", sni);
        let web_key_entropy = AteHash::from_bytes(web_key_entropy.as_bytes());
        let web_key = AuthService::compute_super_key_from_hash(&self.web_key, &web_key_entropy);
        let mut session = AteSessionUser::default();
        session.add_user_read_key(&web_key);
        error!("pre- {}", session);

        // Now gather the rights to the chain
        let session = gather_command(&self.registry, sni.clone(), AteSessionInner::User(session), self.auth_url.clone()).await?;
        error!("post {}", session);

        // Enter a write lock and check again
        let mut guard = self.sessions.write();
        if let Some(ret) = guard.get(sni) {
            return Ok(ret.clone());
        }

        // Cache and and return it
        guard.insert(sni.clone(), session.clone(), self.ttl);
        Ok(session)
    }

    pub async fn get_accessor(&self, host: &str) -> Result<Arc<FileAccessor>, GatherError>
    {
        // Get the session
        let sni = host.to_string();
        let session = self.get_session(&sni).await?;

        // Now get the chain for this host
        let host = host.to_string();
        let chain = {
            let mut chains = self.chains.lock().await;
            if let Some(ret) = chains.remove(&host) {
                chains.insert(host, Arc::clone(&ret), self.ttl);
                ret
            } else {
                let key = ChainKey::from(format!("{}/www", host));
                let chain = self.registry.open(&self.db_url, &key).await?;
                let accessor = Arc::new(
                    FileAccessor::new(
                        chain.as_arc(),
                        Some(host.clone()),
                        AteSessionType::Group(session),
                        TransactionScope::Local,
                        TransactionScope::Local,
                        false,
                        false
                    ).await
                );
                chains.insert(host, Arc::clone(&accessor), self.ttl);
                accessor
            }
        };

        Ok(chain)
    }

    pub async fn get_file(&self, host: &str, path: &str) -> Result<Option<Bytes>, FileSystemError> {
        let path = path.to_string();
        let context = RequestContext::default();
        
        let chain = self.get_accessor(host).await?;
        Ok(
            match chain.search(&context, path.as_str()).await {
                Ok(Some(a)) => {
                    let flags = libc::O_RDONLY as u32;
                    let oh = match chain.open(&context, a.ino, flags).await {
                        Ok(a) => Some(a),
                        Err(FileSystemError(FileSystemErrorKind::IsDirectory, _)) => None,
                        Err(err) => { return Err(err.into()); },
                    };
                    match oh {
                        Some(oh) => Some(
                            chain.read(&context, a.ino, oh.fh, 0, u32::MAX).await?
                        ),
                        None => None
                    }
                },
                Ok(None) => {
                    None
                },
                Err(FileSystemError(FileSystemErrorKind::IsDirectory, _)) |
                Err(FileSystemError(FileSystemErrorKind::DoesNotExist, _)) |
                Err(FileSystemError(FileSystemErrorKind::NoEntry, _)) => {
                    None
                },
                Err(err) => {
                    return Err(err.into());
                }
            }
        )
    }

    pub async fn house_keeping(&self) {
        let mut lock = self.chains.lock().await;
        lock.iter();    // this will run the remove_expired function
    }
}