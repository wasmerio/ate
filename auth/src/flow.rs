#[allow(unused_imports)]
use log::{info, warn, debug, error};
use async_trait::async_trait;
use regex::Regex;
use ate::{error::ChainCreationError, prelude::*};

pub struct ChainFlow {
    root_key: PrivateSignKey,
    regex_auth: Regex,
    regex_cmd: Regex,
}

impl ChainFlow
{
    pub fn new(root_key: PrivateSignKey) -> Self {        
        ChainFlow {
            root_key,
            regex_auth: Regex::new("^/auth-[a-f0-9]{4}$").unwrap(),
            regex_cmd: Regex::new("^/cmd-[a-f0-9]{32}$").unwrap(),
        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    async fn open(&self, cfg: &ConfAte, key: &ChainKey) -> Result<OpenAction, ChainCreationError>
    {
        let name = key.name.clone();
        let name = name.as_str();
        if self.regex_auth.is_match(name) {
            let chain = ChainBuilder::new(cfg)
                .await
                .add_root_public_key(&self.root_key.as_public_key())
                .build(key)
                .await?;
            return Ok(OpenAction::Chain(chain));
        }
        if self.regex_cmd.is_match(name) {
            let chain = ChainBuilder::new(cfg)
                .await
                .temporal(true)
                .build(key)
                .await?;
            return Ok(OpenAction::Chain(chain));
        }
        Ok(OpenAction::Deny(format!("The chain-key ({}) does not match a valid chain supported by this server.", key.to_string()).to_string()))
    }
}