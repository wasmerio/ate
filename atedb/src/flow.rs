#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Arc;
use regex::Regex;

use async_trait::async_trait;
use ate::{error::ChainCreationError, prelude::*};
use ate::spec::TrustMode;
use ate_auth::request::*;

pub struct ChainFlow {
    pub cfg: ConfAte,
    pub regex_personal: Regex,
    pub regex_group: Regex,
    pub mode: TrustMode,
    pub url_auth: Option<url::Url>,
    pub url_db: url::Url,
    pub registry: Arc<Registry>,
}

impl ChainFlow
{
    pub async fn new(cfg: &ConfAte, url_auth: Option<url::Url>, url_db: url::Url, mode: TrustMode) -> Self {
        let registry = ate::mesh::Registry::new(&ate_auth::conf_auth()).await.cement();
        ChainFlow {
            cfg: cfg.clone(),
            regex_personal: Regex::new("^([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})/([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})/([a-zA-Z0-9_]{1,})$").unwrap(),
            regex_group: Regex::new("^([a-zA-Z0-9_\\.-]{1,})/([a-zA-Z0-9_]{1,})$").unwrap(),
            mode,
            url_auth,
            url_db,
            registry,
        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    fn hello_path(&self) -> &str {
        self.url_db.path()
    }

    async fn message_of_the_day(&self, _chain: &Arc<Chain>) -> Result<Option<String>, ChainCreationError>
    {
        Ok(None)
    }

    async fn open(&self, mut builder: ChainBuilder, key: &ChainKey, _wire_encryption: Option<KeySize>) -> Result<OpenAction, ChainCreationError>
    {
        trace!("open_db: {}", key);

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
                return Ok(OpenAction::Deny {
                    reason: format!("The chain-key ({}) contains forbidden characters.", key.to_string()).to_string()
                });
            }

            // Grab the public write key from the authentication server for this user
            if let Some(auth) = &self.url_auth {
                let advert = match ate_auth::query_command(&self.registry, email.clone(), auth.clone()).await {
                    Ok(a) => a.advert,
                    Err(err) => {
                        return Ok(OpenAction::Deny {
                            reason: format!("Failed to create the chain as the query to the authentication server failed - {}.", err.to_string()).to_string()
                        });
                    }
                };
                let root_key = advert.nominal_auth;
                builder = builder.add_root_public_key(&root_key);
            }
            
            // Set the load integrity to match what the database will run as
            builder = builder.load_integrity(self.mode);

            // Prefix the name with 'redo'
            let mut key_name = key.name.clone();
            if key_name.starts_with("/") {
                key_name = key_name[1..].to_string();
            }
            let key = ChainKey::new(format!("{}", key_name).to_string());
            
            // Create the chain
            let chain = builder
                .build()
                .open(&key)
                .await?;

            // We have opened the chain
            return match self.mode.is_centralized() {
                true => {
                    debug!("centralized integrity for {}", key.to_string());
                    Ok(OpenAction::CentralizedChain {
                        chain
                    })
                },
                false => {
                    debug!("distributed integrity for {}", key.to_string());
                    Ok(OpenAction::DistributedChain {
                        chain
                    })
                },
            };
        }

        // The path may match a group that was created
        if let Some(captures) = self.regex_group.captures(path.as_str())
        {
            // Get the auth
            let group = captures.get(1).unwrap().as_str().to_string();
            let dbname = captures.get(2).unwrap().as_str().to_string();

            // Check for very naughty parameters
            if group.contains("..") || dbname.contains("..") || group.contains("~") || dbname.contains("~") {
                return Ok(OpenAction::Deny {
                    reason: format!("The chain-key ({}) contains forbidden characters.", key.to_string()).to_string()
                });
            }

            let auth = match &self.url_auth {
                Some(a) => a.clone(),
                None => {
                    return Ok(OpenAction::Deny {
                        reason: format!("Failed to create the chain for group ({}) as the server has no authentication endpoint configured.", group)
                    });
                }
            };

            // Grab the public write key from the authentication server for this group
            let chain = self.registry.open_cmd(&auth).await?;
            let advert: Result<GroupDetailsResponse, GroupDetailsFailed> = chain.invoke(GroupDetailsRequest {
                group,
                session: None,
            }).await?;
            let advert = match advert {
                Ok(a) => a,
                Err(GroupDetailsFailed::NoAccess) => {
                    return Ok(OpenAction::Deny {
                        reason: format!("Failed to create the chain as the caller has no access to the authorization group({}), contact the owner of the group to add this user to the delegate role of that group.", path)
                    });
                },
                Err(GroupDetailsFailed::NoMasterKey) => {
                    return Ok(OpenAction::Deny {
                        reason: format!("Failed to create the chain as the server has not yet been properly initialized.")
                    });
                },
                Err(GroupDetailsFailed::GroupNotFound) => {
                    return Ok(OpenAction::Deny {
                        reason: format!("Failed to create the chain as no authorization group exists with the same name({}), please create one and try again.", path)
                    });
                },
                Err(GroupDetailsFailed::InternalError(code)) => {
                    return Ok(OpenAction::Deny {
                        reason: format!("Failed to create the chain as the authentication group query failed due to an internal error - code={}.", code)
                    });
                }
            };

            let role = match advert.roles.iter().filter(|r| r.purpose == AteRolePurpose::Delegate).next() {
                Some(a) => a,
                None => { return Ok(OpenAction::Deny {
                    reason: format!("Failed to create the chain as the group has no delegate role.")
                }); }
            };

            builder = builder.add_root_public_key(&role.write);
            
            // Prefix the name with 'redo'
            let mut key_name = key.name.clone();
            if key_name.starts_with("/") {
                key_name = key_name[1..].to_string();
            }
            let key = ChainKey::new(format!("{}", key_name).to_string());

            // Create the chain
            let chain = builder
                .build()
                .open(&key)
                .await?;

            // We have opened the chain
            return match self.mode.is_centralized() {
                true => {
                    debug!("centralized integrity for {}", key.to_string());
                    Ok(OpenAction::CentralizedChain {
                        chain
                    })
                },
                false => {
                    debug!("distributed integrity for {}", key.to_string());
                    Ok(OpenAction::DistributedChain {
                        chain
                    })
                },
            };
        }

        // Ask the authentication server for the public key for this user
        return Ok(OpenAction::Deny {
            reason: format!("The chain-key ({}) does not match a valid pattern - for private datachains it must be in the format of 'gmail.com/joe.blogs/mydb' where the owner of this chain is the user joe.blogs@gmail.com. - for shared datachains you must first create a group with the same name and then use the format '[group-name]/[chain-name]'.", key.to_string()).to_string()
        });
    }
}