#[allow(unused_imports)]
use log::{info, warn, debug, error};
use std::sync::Arc;
use regex::Regex;

use async_trait::async_trait;
use ate::{error::ChainCreationError, prelude::*};
use ate::trust::IntegrityMode;
use ate_auth::commands::*;

pub struct ChainFlow {
    cfg: ConfAte,
    regex_personal: Regex,
    regex_group: Regex,
    mode: TrustMode,
    auth: Option<url::Url>,
    registry: Arc<Registry>,
}

impl ChainFlow
{
    pub async fn new(cfg: &ConfAte, auth: Option<url::Url>, mode: TrustMode) -> Self {        
        ChainFlow {
            cfg: cfg.clone(),
            regex_personal: Regex::new("^/([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})/([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})/([a-zA-Z0-9_]{1,})$").unwrap(),
            regex_group: Regex::new("^/{0,1}([a-zA-Z0-9_]{0,})$").unwrap(),
            mode,
            auth,
            registry: ate::mesh::Registry::new(&ate_auth::conf_auth(), true).await
        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    async fn open(&self, mut builder: ChainBuilder, key: &ChainKey) -> Result<OpenAction, ChainCreationError>
    {
        // Extract the identity from the supplied path (we can only create chains that are actually
        // owned by the specific user)
        let path = key.name.clone();
        if let Some(captures) = self.regex_personal.captures(path.as_str())
        {
            // Build the email address using the captures
            let email = format!("{}@{}", captures.get(2).unwrap().as_str(), captures.get(1).unwrap().as_str());
            let dbname = captures.get(3).unwrap().as_str().to_string();

            // Check for very naughty parameters
            if email.contains("..") || dbname.contains("..") || email.contains("~") || dbname.contains("~") {
                return Ok(OpenAction::Deny(format!("The chain-key ({}) contains forbidden characters.", key.to_string()).to_string()));
            }

            // Grab the public write key from the authentication server for this user
            if let Some(auth) = &self.auth {
                let advert = match ate_auth::query_command(Arc::clone(&self.registry), email.clone(), auth.clone()).await {
                    Ok(a) => a.advert,
                    Err(err) => {
                        return Ok(OpenAction::Deny(format!("Failed to create the chain as the query to the authentication server failed - {}.", err.to_string()).to_string()));
                    }
                };
                let root_key = advert.nominal_auth;
                builder = builder.add_root_public_key(&root_key);
            }

            builder = builder.integrity(match self.mode {
                TrustMode::Centralized => {
                    debug!("centralized integrity for {}", key.to_string());
                    IntegrityMode::Centralized
                },
                TrustMode::Distributed => {
                    debug!("distributed integrity for {}", key.to_string());
                    IntegrityMode::Distributed
                }
            });
            
            let chain = builder
                .build()
                .open(key)
                .await?;

            // We have opened the chain
            return match self.mode {
                TrustMode::Centralized => Ok(OpenAction::CentralizedChain(chain)),
                TrustMode::Distributed => Ok(OpenAction::DistributedChain(chain)),
            };
        }

        // The path may match a group that was created
        if let Some(_captures) = self.regex_group.captures(path.as_str())
        {
            // Get the auth
            let group = path.clone();
            let auth = match &self.auth {
                Some(a) => a.clone(),
                None => {
                    return Ok(OpenAction::Deny(format!("Failed to create the chain for group ({}) as the server has no authentication endpoint configured.", group)));
                }
            };

            // Prepare the right chain            
            let chain_url = ate_auth::command_url(auth.clone());
            let chain = Arc::clone(&self.registry).open_by_url(&chain_url).await?;

            // Grab the public write key from the authentication server for this group
            let advert: Result<GroupDetailsResponse, InvokeError<GroupDetailsFailed>> = chain.invoke(GroupDetailsRequest {
                group,
                session: None,
            }).await;
            let advert = match advert {
                Ok(a) => a,
                Err(InvokeError::Reply(GroupDetailsFailed::NoAccess)) => {
                    return Ok(OpenAction::Deny(format!("Failed to create the chain as the caller has no access to the group({}).", path)));
                },
                Err(InvokeError::Reply(GroupDetailsFailed::GroupNotFound)) => {
                    return Ok(OpenAction::Deny(format!("Failed to create the chain as no group exists with the same name({}).", path)));
                },
                Err(err) => {
                    return Ok(OpenAction::Deny(format!("Failed to create the chain as the group query failed - {}.", err)));
                }
            };

            let role = match advert.roles.iter().filter(|r| r.purpose == AteRolePurpose::Delegate).next() {
                Some(a) => a,
                None => { return Ok(OpenAction::Deny(format!("Failed to create the chain as the group has no delegate role."))); }
            };

            builder = builder.add_root_public_key(&role.write);
            builder = builder.integrity(match self.mode {
                TrustMode::Centralized => {
                    debug!("centralized integrity for {}", key.to_string());
                    IntegrityMode::Centralized
                },
                TrustMode::Distributed => {
                    debug!("distributed integrity for {}", key.to_string());
                    IntegrityMode::Distributed
                }
            });
            
            let key = ChainKey::new(format!("group.{}", key.name).to_string());
            let chain = builder
                .build()
                .open(&key)
                .await?;

            // We have opened the chain
            return match self.mode {
                TrustMode::Centralized => Ok(OpenAction::CentralizedChain(chain)),
                TrustMode::Distributed => Ok(OpenAction::DistributedChain(chain)),
            };
        }

        // Ask the authentication server for the public key for this user
        return Ok(OpenAction::Deny(format!("The chain-key ({}) does not match a valid pattern - for private databases it must be in the format of /gmail.com/joe.blogs/mydb where the owner of this chain is the user joe.blogs@gmail.com. - for shared databases you must first create a group with the same name.", key.to_string()).to_string()));
    }
}