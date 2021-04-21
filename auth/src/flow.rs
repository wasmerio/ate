#[allow(unused_imports)]
use log::{info, warn, debug, error};
use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;
use ate::{error::ChainCreationError, prelude::*};
use ate::trust::IntegrityMode;

use crate::service::*;

pub struct ChainFlow {
    cfg: ConfAte,
    root_key: PrivateSignKey,
    regex_auth: Regex,
    regex_cmd: Regex,
    session: AteSession,
}

impl ChainFlow
{
    pub fn new(cfg: &ConfAte, root_key: PrivateSignKey, session: AteSession) -> Self {        
        ChainFlow {
            cfg: cfg.clone(),
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
                .set_session(self.session.clone())
                .add_root_public_key(&self.root_key.as_public_key())
                .build()
                .open(key)
                .await?;

            return Ok(OpenAction::DistributedChain(chain));
        }
        if self.regex_cmd.is_match(name)
        {
            // Build a secure session
            let session_root_key = PrivateSignKey::generate(KeySize::Bit128);
            let mut cmd_session = AteSession::default();
            cmd_session.user.add_read_key(&EncryptKey::generate(KeySize::Bit128));
            cmd_session.user.add_write_key(&session_root_key);

            // Build the chain
            let chain = builder
                .integrity(IntegrityMode::Distributed)
                .set_session(cmd_session.clone())
                .add_root_public_key(&session_root_key.as_public_key())
                .temporal(true)
                .build()
                .open(key)
                .await?;
                
            // Add the services to this chain
            service_logins(&self.cfg, cmd_session.clone(), self.session.clone(), &Arc::clone(&chain)).await?;

            // Return the chain to the caller
            return Ok(OpenAction::PrivateChain
            {
                chain,
                session: cmd_session
            });
        }
        Ok(OpenAction::Deny(format!("The chain-key ({}) does not match a valid chain supported by this server.", key.to_string()).to_string()))
    }
}