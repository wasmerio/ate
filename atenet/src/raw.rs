use std::sync::Arc;
use tokio::sync::broadcast;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use smoltcp::wire::EthernetFrame;

use super::switch::Switch;

#[derive(Debug)]
pub struct TapSocket
{
    switch: Arc<Switch>,
    recv: broadcast::Receiver<Vec<u8>>,
}

impl TapSocket
{
    pub fn new(switch: &Arc<Switch>, rx: broadcast::Receiver<Vec<u8>>) -> TapSocket {
        TapSocket {
            switch: Arc::clone(switch),
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
}