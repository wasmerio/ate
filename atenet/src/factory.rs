use std::sync::Arc;
use std::sync::Weak;
use std::sync::RwLock;
use std::collections::HashMap;
use ate::prelude::*;
use ate_files::repo::Repository;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::switch::Switch;
use super::udp::UdpPeer;
use super::gateway::Gateway;

/// Factory that gets and creates switches
#[derive(Debug)]
pub struct SwitchFactory
{
    switches: RwLock<HashMap<u128, Weak<Switch>>>,
    repo: Arc<Repository>,
    udp: UdpPeer,
    instance_authority: String,
}

impl SwitchFactory
{
    pub fn new(repo: Arc<Repository>, udp: UdpPeer, instance_authority: String) -> SwitchFactory {
        SwitchFactory {
            switches: Default::default(),
            repo,
            udp,
            instance_authority
        }
    }
    pub async fn get_or_create_switch(self: &Arc<SwitchFactory>, id: u128, key: ChainKey) -> Result<(Arc<Switch>, bool), CommsError> {
        // Check the cache
        {
            let guard = self.switches.read().unwrap();
            if let Some(ret) = guard.get(&id) {
                if let Some(ret) = ret.upgrade() {
                    return Ok((ret, false));
                }
            }
        }

        // Open the instance chain that backs this particular instance
        // (this will reuse accessors across threads and calls)
        let accessor = self.repo.get_accessor(&key, self.instance_authority.as_str()).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;
        trace!("loaded file accessor for {}", key);

        // Create the gateway
        let gateway = Arc::new(Gateway::new(id, self));

        // Build the switch
        let switch = Switch::new(accessor, self.udp.clone(), gateway).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;

        // Enter a write lock and check again
        let mut guard = self.switches.write().unwrap();
        if let Some(ret) = guard.get(&id) {
            if let Some(ret) = ret.upgrade() {
                return Ok((ret, false));
            }
        }

        // Cache and and return it
        guard.insert(id, Arc::downgrade(&switch));
        Ok((switch, true))
    }
}