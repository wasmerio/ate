use std::sync::Arc;
use std::sync::Weak;
use std::sync::RwLock;
use std::collections::HashMap;
use ate::prelude::*;
use ate_files::repo::Repository;
use smoltcp::wire::IpAddress;
use wasmer_deploy_cli::model::ServiceInstance;
use wasmer_deploy_cli::model::INSTANCE_ROOT_ID;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::switch::Switch;
use super::udp::UdpPeerHandle;
use super::gateway::Gateway;

/// Factory that gets and creates switches
#[derive(Debug)]
pub struct SwitchFactory
{
    switches_by_key: Arc<RwLock<HashMap<ChainKey, Weak<Switch>>>>,
    switches_by_id: Arc<RwLock<HashMap<u128, Weak<Switch>>>>,
    repo: Arc<Repository>,
    udp: UdpPeerHandle,
    instance_authority: String,
}

impl SwitchFactory
{
    pub fn new(repo: Arc<Repository>, udp: UdpPeerHandle, instance_authority: String, switches: Arc<RwLock<HashMap<u128, Weak<Switch>>>>) -> SwitchFactory {
        SwitchFactory {
            switches_by_key: Default::default(),
            switches_by_id: switches,
            repo,
            udp,
            instance_authority
        }
    }
    pub async fn get_or_create_switch(self: &Arc<SwitchFactory>, key: ChainKey) -> Result<(Arc<Switch>, bool), CommsError> {
        // Check the cache
        {
            let guard = self.switches_by_key.read().unwrap();
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
        debug!("loaded file accessor for {}", key);

        // Create the gateway
        let inst = accessor.dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;
        let id = inst.id;
        let gateway_ips= inst.subnet.cidrs.iter()
            .map(|cidr| {
                let ip: IpAddress = cidr.gateway().into();
                ip
            })
            .filter(|ip| ip.is_unicast())
            .collect();
        let gateway = Arc::new(Gateway::new(id, gateway_ips, self));

        // Build the switch
        let cidrs = super::common::subnet_to_cidrs(&inst.subnet);
        let switch = Switch::new(accessor, cidrs, self.udp.clone(), gateway).await
            .map_err(|err| CommsErrorKind::InternalError(err.to_string()))?;

        // Every one thousand or so switches we will clean the orphans
        let clean = fastrand::u16(0..1000) == 1;

        // Enter a write lock and check again
        let mut guard = self.switches_by_key.write().unwrap();
        if let Some(ret) = guard.get(&key) {
            if let Some(ret) = ret.upgrade() {
                return Ok((ret, false));
            }
        }

        // We also need to write the switch to the ID referenced one
        {
            let mut guard = self.switches_by_id.write().unwrap();
            if clean { guard.retain(|_, v| v.strong_count() >= 1); }
            guard.insert(id, Arc::downgrade(&switch));
        }

        // Cache and and return it
        if clean { guard.retain(|_, v| v.strong_count() >= 1); }
        guard.insert(key, Arc::downgrade(&switch));
        Ok((switch, true))
    }
}