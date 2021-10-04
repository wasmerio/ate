#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Arc;
use std::ops::Deref;

use ate::prelude::*;

use crate::error::*;
use crate::helper::b64_to_session;
use crate::cmd::main_session_prompt;
use crate::cmd::gather_command;

pub struct DioBuilder
{
    cfg_ate: ConfAte,
    url_db: url::Url,
    url_auth: url::Url,
    registry: Option<Arc<Registry>>,
    session: Box<dyn AteSession>,
    group: Option<String>,
}

impl Default
for DioBuilder
{
    fn default() -> DioBuilder {
        DioBuilder {
            cfg_ate: ConfAte::default(),
            url_db: url::Url::parse("ws://tokera.com/db").unwrap(),
            url_auth: url::Url::parse("ws://tokera.com/auth").unwrap(),
            registry: None,
            session: Box::new(AteSessionUser::default()),
            group: None,
        }
    }
}

impl DioBuilder
{
    pub fn cfg<'a>(&'a mut self) -> &'a mut ConfAte {
        &mut self.cfg_ate
    }

    pub fn with_cfg_ate(mut self, cfg: ConfAte) -> Self {
        self.cfg_ate = cfg;
        self
    }

    pub fn with_url_db(mut self, url: url::Url) -> Self {
        self.url_db = url;
        self
    }

    pub fn with_url_auth(mut self, url: url::Url) -> Self {
        self.url_auth = url;
        self.registry = None;
        self
    }

    pub fn with_token_string(mut self, token: String) -> Self {
        self.session = Box::new(b64_to_session(token));
        self
    }

    pub async fn with_token_path(mut self, path: String) -> Result<Self, LoginError> {
        let path = shellexpand::tilde(path.as_str()).to_string();
        let token = tokio::fs::read_to_string(path).await?;
        self.session = Box::new(b64_to_session(token));
        Ok(self)
    }

    pub fn with_registry(mut self, registry: Registry) -> Self {
        self.registry = Some(Arc::new(registry));
        self
    }

    pub async fn get_registry(&mut self) -> Arc<Registry>
    {
        if self.registry.is_none() {
            self.registry = Some(Arc::new(Registry::new(&self.cfg_ate).await));
        }
        Arc::clone(self.registry.as_ref().unwrap())
    }

    pub fn with_session(mut self, session: Box<dyn AteSession>) -> Self {
        self.session = session;
        self
    }

    pub async fn with_session_prompt(mut self) -> Result<Self, LoginError> {
        self.session = Box::new(main_session_prompt(self.url_auth.clone()).await?);
        Ok(self)
    }

    pub async fn with_group(mut self, group: &str) -> Result<Self, GatherError> {
        let registry = self.get_registry().await;
        let session = gather_command(&registry, group.to_string(), self.session.clone_inner(), self.url_auth.clone()).await?;
        self.session = Box::new(session);
        self.group = Some(group.to_string());
        Ok(self)
    }

    pub fn generate_key(&self, name: &str) -> String
    {
        match &self.group {
            Some(a) => format!("{}/{}", a, name),
            None => {
                let identity = self.session.identity();
                let comps = identity.split("@").collect::<Vec<_>>();
                if comps.len() >= 2 {
                    let user = comps[0];
                    let domain = comps[1];
                    format!("{}/{}/{}", domain, user, name)
                } else {
                    name.to_string()
                }
            }
        }
    }

    pub async fn build(&mut self, name: &str) -> Result<Arc<DioMut>, LoginError> {
        
        let key = ChainKey::new(self.generate_key(name));
        let registry = self.get_registry().await;
        let chain = registry.open(&self.url_db, &key).await?;
        Ok(chain.dio_mut(self.session.deref()).await)
    }
}