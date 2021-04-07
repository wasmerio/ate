#[allow(unused_imports)]
use log::{info, warn, debug, error};
use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;
use ate::{error::ChainCreationError, prelude::*};

use crate::service::*;

pub struct ChainFlow {
    root_key: PrivateSignKey,
    regex_auth: Regex,
    regex_cmd: Regex,
    session: AteSession,
}

impl ChainFlow
{
    pub fn new(root_key: PrivateSignKey, session: AteSession) -> Self {        
        ChainFlow {
            root_key,
            regex_auth: Regex::new("^/auth-[a-f0-9]{4}$").unwrap(),
            regex_cmd: Regex::new("^/cmd-[a-f0-9]{16}$").unwrap(),
            session,
        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    async fn open(&self, builder: ChainBuilder, key: &ChainKey) -> Result<OpenAction, ChainCreationError>
    {
        let name = key.name.clone();
        let name = name.as_str();
        if self.regex_auth.is_match(name) {
            let chain = builder
                .add_root_public_key(&self.root_key.as_public_key())
                .build(key)
                .await?;
            let chain = Arc::new(chain);

            return Ok(OpenAction::Chain(chain));
        }
        if self.regex_cmd.is_match(name) {
            let chain = builder
                .temporal(true)
                .build(key)
                .await?;
            let chain = Arc::new(chain);

            // Add the services to this chain
            service_logins(self.session.clone(), &Arc::clone(&chain)).await;

            // Return the chain to the caller
            return Ok(OpenAction::Chain(chain));
        }
        Ok(OpenAction::Deny(format!("The chain-key ({}) does not match a valid chain supported by this server.", key.to_string()).to_string()))
    }
}