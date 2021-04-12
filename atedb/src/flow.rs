#[allow(unused_imports)]
use log::{info, warn, debug, error};
use std::sync::Arc;
use regex::Regex;

use async_trait::async_trait;
use ate::{error::ChainCreationError, prelude::*};

pub struct ChainFlow {
    cfg: ConfAte,
    regex: Regex,
    auth: url::Url,
}

impl ChainFlow
{
    pub fn new(cfg: &ConfAte, auth: url::Url) -> Self {        
        ChainFlow {
            cfg: cfg.clone(),
            regex: Regex::new("^/([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})/([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})/([a-zA-Z0-9_]{1,})$").unwrap(),
            auth,
        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    async fn open(&self, builder: ChainBuilder, key: &ChainKey) -> Result<OpenAction, ChainCreationError>
    {
        // Extract the identity from the supplied path (we can only create chains that are actually
        // owned by the specific user)
        let path = key.name.clone();
        if let Some(captures) = self.regex.captures(path.as_str())
        {
            // Build the email address using the captures
            let email = format!("{}@{}", captures.get(2).unwrap().as_str(), captures.get(1).unwrap().as_str());
            let dbname = captures.get(3).unwrap().as_str().to_string();

            // Check for very naughty parameters
            if email.contains("..") || dbname.contains("..") || email.contains("~") || dbname.contains("~") {
                return Ok(OpenAction::Deny(format!("The chain-key ({}) contains forbidden characters.", key.to_string()).to_string()));
            }

            // Grab the public write key from the authentication server for this user
            let advert = match ate_auth::query_command(email.clone(), self.auth.clone()).await {
                Ok(a) => a.advert,
                Err(err) => {
                    return Ok(OpenAction::Deny(format!("Failed to create the chain as the query to the authentication server failed - {}.", err.to_string()).to_string()));
                }
            };
            let root_key = advert.auth;

            let chain = builder
                .add_root_public_key(&root_key)
                .build(key)
                .await?;

            // Build a secure session and the chain
            let chain = Arc::new(chain);

            // We have opened the chain
            return Ok(OpenAction::CentralizedChain(chain));
        }

        // Ask the authentication server for the public key for this user
        return Ok(OpenAction::Deny(format!("The chain-key ({}) does not match a valid pattern - it must be in the format of /gmail.com/joe.blogs/mydb where the owner of this chain is the user joe.blogs@gmail.com.", key.to_string()).to_string()));
    }
}