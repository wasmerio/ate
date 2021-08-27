#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;
use ate::{error::ChainCreationError, prelude::*};
use ate::crypto::EncryptKey;
use ate::crypto::KeySize;

use crate::service::*;

pub struct ChainFlow {
    cfg: ConfAte,
    auth_url: url::Url,
    root_key: PrivateSignKey,
    regex_auth: Regex,
    regex_cmd: Regex,
    session: AteSession,
    pub terms_and_conditions: Option<String>,
}

impl ChainFlow
{
    pub fn new(cfg: &ConfAte, root_key: PrivateSignKey, session: AteSession, auth_url: &url::Url) -> Self {        
        ChainFlow {
            cfg: cfg.clone(),
            root_key,
            regex_auth: Regex::new("^redo-[a-f0-9]{4}$").unwrap(),
            regex_cmd: Regex::new("^cmd-[a-f0-9]{16}$").unwrap(),
            auth_url: auth_url.clone(),
            session,
            terms_and_conditions: None,
        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    fn hello_path(&self) -> &str {
        self.auth_url.path()
    }

    async fn message_of_the_day(&self, _chain: &Arc<Chain>) -> Result<Option<String>, ChainCreationError> {
        Ok(None)
    }

    async fn open(&self, mut builder: ChainBuilder, key: &ChainKey, wire_encryption: Option<KeySize>) -> Result<OpenAction, ChainCreationError>
    {
        debug!("open_auth: {}", key);

        let name = key.name.clone();
        let name = name.as_str();
        if self.regex_auth.is_match(name)
        {
            let chain = builder
                .set_session(self.session.clone())
                .add_root_public_key(&self.root_key.as_public_key())
                .build()
                .open(key)
                .await?;

            return Ok(OpenAction::DistributedChain {
                chain: chain,
            });
        }
        if self.regex_cmd.is_match(name)
        {
            // Build a secure session
            let mut cmd_session = AteSession::default();
            cmd_session.user.add_read_key(&EncryptKey::generate(KeySize::Bit128));

            // For command based chains that are already encryption there is no need
            // to also add signatures which take lots of CPU power
            let session_root_key = if wire_encryption.is_none() {
                let key = PrivateSignKey::generate(KeySize::Bit128);
                cmd_session.user.add_write_key(&key);
                Some(key)
            } else {
                None
            };

            // Build the chain
            builder = builder
                .set_session(cmd_session.clone())
                .temporal(true);
            if let Some(session_root_key) = &session_root_key {
                builder = builder.add_root_public_key(&session_root_key.as_public_key())
            }

            let chain = builder
                .build()
                .open(key)
                .await?;
                
            // Add the services to this chain
            service_auth_handlers(&self.cfg, cmd_session.clone(), self.auth_url.clone(), self.session.clone(), self.terms_and_conditions.clone(), &Arc::clone(&chain)).await?;

            // Return the chain to the caller
            return Ok(OpenAction::PrivateChain
            {
                chain,
                session: cmd_session,
            });
        }
        Ok(OpenAction::Deny {
            reason: format!("The chain-key ({}) does not match a valid chain supported by this server.", key.to_string()).to_string()
        })
    }
}