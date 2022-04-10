use std::sync::Arc;
use std::sync::Weak;
use std::sync::RwLock;
use std::collections::HashMap;
use ate::prelude::*;
use ate_files::repo::Repository;
use tokera::model::ServiceInstance;
use tokera::model::INSTANCE_ROOT_ID;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::switch::Switch;
use super::udp::UdpPeer;
use super::gateway::Gateway;

/// Factory that gets and creates switches
#[derive(Debug)]
pub struct SwitchFactory
{
    switches: RwLock<HashMap<ChainKey, Weak<Switch>>>,
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
    pub async fn get_or_create_switch(self: &Arc<SwitchFactory>, key: ChainKey) -> Result<(Arc<Switch>, bool), CommsError> {
        // Check the cache
        {
            let guard = self.switches.read().unwrap();
            if let Some(ret) = guard.get(&key) {
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
        let inst = accessor.dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;
        let id = inst.id;
        let gateway_ips = inst.subnet.cidrs.iter()
            .map(|cidr| cidr.gateway().into())
            .collect();
        let gateway = Arc::new(Gateway::new(id, gateway_ips, self));

        // Build the switch
        let cidrs = inst.subnet.cidrs.iter()
            .map(|cidr| smoltcp::wire::IpCidr::new(cidr.ip.into(), cidr.prefix))
            .collect();
        let switch = Switch::new(accessor, cidrs, self.udp.clone(), gateway).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;

        // Enter a write lock and check again
        let mut guard = self.switches.write().unwrap();
        if let Some(ret) = guard.get(&key) {
            if let Some(ret) = ret.upgrade() {
                return Ok((ret, false));
            }
        }

        // Cache and and return it
        guard.insert(key, Arc::downgrade(&switch));
        Ok((switch, true))
    }
}