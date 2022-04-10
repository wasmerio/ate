use std::sync::Arc;
use std::sync::Weak;
use std::sync::RwLock;
use std::collections::HashMap;
use tokera::model::IpCidr;

use super::switch::Switch;
use super::factory::SwitchFactory;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Route
{
    id: u128,
    cidr: IpCidr,
    switch: Weak<Switch>,
    access_code: String,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct GatewayState
{
    routes: HashMap<IpCidr, Route>
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Gateway
{
    id: u128,
    state: RwLock<GatewayState>,
    factory: Arc<SwitchFactory>,
}

impl Gateway
{
    pub fn new(id: u128, factory: &Arc<SwitchFactory>) -> Gateway {
        Gateway {
            id,
            state: Default::default(),
            factory: factory.clone(),
        }
    }

    pub fn process_outbound(&self, _pck: &[u8]) {

    }

    pub fn process_inbound(&self, _pck: &[u8]) {

    }
}