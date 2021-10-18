#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use std::sync::Arc;
use std::sync::Weak;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use parking_lot::Mutex as StdMutex;
use tokio::sync::broadcast;

use crate::error::*;
use crate::prelude::SerializationFormat;
use crate::crypto::EncryptKey;

use super::NodeId;
use super::conf::Upstream;
use super::Packet;
use super::PacketData;
use super::PacketWithContext;
use super::Metrics;
use super::Throttle;

#[derive(Debug)]
pub(crate) enum TxDirection
{
    #[cfg(feature="enable_server")]
    Downcast(TxGroupSpecific),
    Upcast(Upstream),
    #[allow(dead_code)]
    Nullcast,
}

#[derive(Debug)]
pub(crate) struct TxRelay
{
    pub direction: TxDirection,
    pub wire_format: SerializationFormat,
}

#[derive(Debug)]
pub(crate) struct Tx
{
    pub hello_path: String,
    pub(crate) direction: TxDirection,
    pub wire_format: SerializationFormat,
    pub(crate) relay: Option<TxRelay>,
    pub metrics: Arc<StdMutex<Metrics>>,
    pub throttle: Arc<StdMutex<Throttle>>,
    pub(crate) exit_dependencies: Vec<broadcast::Sender<()>>,
}

impl Tx
{
    #[allow(dead_code)]
    pub async fn send_relay<M, C>(&mut self, pck: PacketWithContext<M, C>) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
          C: Send + Sync,
    {
        let mut total_sent = 0u64;
        if let Some(relay) = self.relay.as_mut() {
            let pck = if self.wire_format == relay.wire_format {
                pck.data
            } else {
                Packet::from(pck.packet.msg).to_packet_data(relay.wire_format)?
            };
            match &mut relay.direction {
                #[cfg(feature="enable_server")]
                TxDirection::Downcast(tx) => {
                    total_sent += tx.send_reply(pck).await?;
                },
                TxDirection::Upcast(tx) => {
                    total_sent += tx.outbox.send(pck).await?;
                },
                TxDirection::Nullcast => {
                }
            }
        }
        self.metrics_add_sent(total_sent).await;
        Ok(())
    }

    pub async fn send_reply(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let total_sent = match &mut self.direction {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(tx) => {
                tx.send_reply(pck).await?
            },
            TxDirection::Upcast(tx) => {
                tx.outbox.send(pck).await?
            },
            TxDirection::Nullcast => {
                0u64
            }
        };
        self.metrics_add_sent(total_sent).await;
        Ok(())
    }

    pub async fn send_reply_msg<M>(&mut self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone
    {
        let pck = Packet::from(msg).to_packet_data(self.wire_format)?;
        self.send_reply(pck).await?;
        Ok(())
    }

    #[cfg(feature="enable_server")]
    pub async fn send_others(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let total_sent = match &mut self.direction {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(tx) => {
                tx.send_others(pck).await?
            },
            _ => 0u64
        };
        self.metrics_add_sent(total_sent).await;
        Ok(())
    }

    pub async fn send_all(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let total_sent = match &mut self.direction {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(tx) => {
                tx.send_all(pck).await?
            },
            TxDirection::Upcast(tx) => {
                tx.outbox.send(pck).await?
            },
            TxDirection::Nullcast => {
                0u64
            }
        };
        self.metrics_add_sent(total_sent).await;
        Ok(())
    }

    pub async fn send_all_msg<M>(&mut self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone
    {
        let pck = Packet::from(msg).to_packet_data(self.wire_format)?;
        self.send_all(pck).await?;
        Ok(())
    }

    #[cfg(feature="enable_server")]
    pub(crate) async fn replace_group(&mut self, new_group: Arc<Mutex<TxGroup>>)
    {
        match &mut self.direction {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(tx) => {
                {
                    let mut new_group = new_group.lock().await;
                    new_group.all.insert(tx.me_id, Arc::downgrade(&tx.me_tx));
                }

                let old_group = tx.replace_group(new_group);

                {
                    let mut old_group = old_group.lock().await;
                    old_group.all.remove(&tx.me_id);
                }
            }
            _ => { }
        };
    }

    #[allow(dead_code)]
    pub fn take(&mut self) -> Tx
    {
        let mut direction = TxDirection::Nullcast;
        std::mem::swap(&mut self.direction, &mut direction);

        let ret = Tx {
            hello_path: self.hello_path.clone(),
            direction,
            wire_format: self.wire_format.clone(),
            relay: None,
            metrics: Arc::clone(&self.metrics),
            throttle: Arc::clone(&self.throttle),
            exit_dependencies: Vec::new(),
        };
        ret
    }

    #[allow(dead_code)]
    pub fn set_relay(&mut self, mut tx: Tx)
    {
        let mut direction = TxDirection::Nullcast;
        std::mem::swap(&mut tx.direction, &mut direction);

        self.relay.replace(TxRelay {
            direction,
            wire_format: tx.wire_format
        });
    }

    #[allow(dead_code)]
    pub fn relay_is_some(&self) -> bool
    {
        self.relay.is_some()
    }

    async fn metrics_add_sent(&self, amt: u64)
    {
        // Update the metrics with all this received data
        let mut metrics = self.metrics.lock();
        metrics.sent += amt;
    }

    #[allow(dead_code)]
    pub async fn wire_encryption(&self) -> Option<EncryptKey>
    {
        self.direction.wire_encryption().await
    }

    #[allow(dead_code)]
    pub fn add_exit_dependency(&mut self, exit: broadcast::Sender<()>) {
        self.exit_dependencies.push(exit);
    }
}

impl Drop
for Tx
{
    fn drop(&mut self)
    {
        for exit in self.exit_dependencies.drain(..) {
            let _ = exit.send(());
        }

        #[cfg(feature = "enable_super_verbose")]
        trace!("drop(node-tx)");
    }
}

#[derive(Debug)]
pub(crate) struct TxGroupSpecific
{
    pub me_id: NodeId,
    pub me_tx: Arc<Mutex<Upstream>>,
    pub group: Arc<Mutex<TxGroup>>,
}

impl TxGroupSpecific
{
    #[must_use="all network communication metrics must be accounted for"]
    #[cfg(feature="enable_server")]
    pub async fn send_reply(&mut self, pck: PacketData) -> Result<u64, CommsError>
    {
        let mut tx = self.me_tx.lock().await;
        let total_sent = tx.outbox.send(pck).await?;
        Ok(total_sent)
    }

    #[must_use="all network communication metrics must be accounted for"]
    #[cfg(feature="enable_server")]
    pub async fn send_others(&mut self, pck: PacketData) -> Result<u64, CommsError>
    {
        let mut group = self.group.lock().await;
        let total_sent = group.send(pck, Some(self.me_id)).await?;
        Ok(total_sent)
    }

    #[must_use="all network communication metrics must be accounted for"]
    #[cfg(feature="enable_server")]
    pub async fn send_all(&mut self, pck: PacketData) -> Result<u64, CommsError>
    {
        let mut group = self.group.lock().await;
        let total_sent = group.send(pck, None).await?;
        Ok(total_sent)
    }

    #[cfg(feature="enable_server")]
    pub(crate) fn replace_group(&mut self, group: Arc<Mutex<TxGroup>>) -> Arc<Mutex<TxGroup>>
    {
        std::mem::replace(&mut self.group, group)
    }

    #[allow(dead_code)]
    pub async fn wire_encryption(&self) -> Option<EncryptKey>
    {
        let guard = self.me_tx.lock().await;
        guard.wire_encryption()
    }
}

#[derive(Debug, Default)]
pub(crate) struct TxGroup
{
    pub all: FxHashMap<NodeId, Weak<Mutex<Upstream>>>,
}

impl TxGroup
{
    #[must_use="all network communication metrics must be accounted for"]
    #[cfg(feature="enable_server")]
    pub(crate) async fn send(&mut self, pck: PacketData, skip: Option<NodeId>) -> Result<u64, CommsError>
    {
        let mut total_sent = 0u64;
        match self.all.len() {
            1 => {
                if let Some(tx) = self.all.values().next().iter().filter_map(|a| Weak::upgrade(a)).next() {
                    let mut tx = tx.lock().await;
                    if Some(tx.id) != skip {
                        total_sent += tx.outbox.send(pck).await?;
                    }
                }
            },
            _ => {
                let all = self.all.values().filter_map(|a| Weak::upgrade(a));
                for tx in all {
                    let mut tx = tx.lock().await;
                    if Some(tx.id) != skip {
                        total_sent += tx.outbox.send(pck.clone()).await?;
                    }
                }
            }
        }
        Ok(total_sent)
    }
}

impl TxDirection
{
    #[allow(dead_code)]
    pub async fn wire_encryption(&self) -> Option<EncryptKey>
    {
        match self {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(a) => a.wire_encryption().await,
            TxDirection::Nullcast => None,
            TxDirection::Upcast(a) => a.wire_encryption(),
        }
    }
}