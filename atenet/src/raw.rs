use std::sync::Arc;
use tokio::sync::broadcast;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use smoltcp::wire::EthernetFrame;
use tokera::model::HardwareAddress;

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
        if let Ok(frame_mac) = EthernetFrame::new_checked(&data[..]) {
            let src = frame_mac.src_addr();
            let dst = frame_mac.dst_addr();
            drop(frame_mac);

            if dst.is_unicast() {
                self.switch.unicast(&src, &dst, data, true, None);
            } else {
                self.switch.broadcast(&src, data, true, None);
            }
        } else {
            trace!("dropped invalid packet (len={})", data.len());
        }
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