use std::sync::Arc;
use tokio::sync::broadcast;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use wasmer_deploy_cli::model::HardwareAddress;

use super::switch::Switch;

#[derive(Debug)]
pub struct TapSocket
{
    switch: Arc<Switch>,
    mac: HardwareAddress,
    recv: broadcast::Receiver<Vec<u8>>,
}

impl TapSocket
{
    pub fn new(switch: &Arc<Switch>, mac: HardwareAddress, rx: broadcast::Receiver<Vec<u8>>) -> TapSocket {
        TapSocket {
            switch: Arc::clone(switch),
            mac,
            recv: rx,
        }
    }

    pub fn send(&self, data: Vec<u8>) {
        self.switch.process(data, true, None);
    }

    pub fn recv(&mut self) -> Option<Vec<u8>> {
        self.recv.try_recv().ok()
    }

    pub fn set_promiscuous(&mut self, promiscuous: bool) {
        let mac = self.mac.clone();
        let switch = self.switch.clone();
        tokio::task::spawn(async move {
            if let Err(err) = switch.set_promiscuous(mac, promiscuous).await {
                warn!("failed to set promiscuous mode - {}", err);
            }
        });
    }
}