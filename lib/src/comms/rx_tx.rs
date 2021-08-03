#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use std::sync::Arc;
use std::sync::Weak;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use parking_lot::Mutex as StdMutex;

use crate::error::*;
use crate::prelude::SerializationFormat;

use super::conf::Upstream;
use super::Packet;
use super::PacketData;

#[derive(Debug)]
pub(crate) enum TxDirection
{
    #[cfg(feature="enable_server")]
    Downcast(TxGroupSpecific),
    Upcast(Upstream),
}

#[derive(Debug)]
pub(crate) struct Tx
{
    pub hello_path: String,
    pub direction: TxDirection,
    pub wire_format: SerializationFormat,
}

impl Tx
{
    pub async fn send_reply(&mut self, pck: PacketData) -> Result<(), CommsError> {
        match &mut self.direction {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(tx) => {
                tx.send_reply(pck).await?;
            },
            TxDirection::Upcast(tx) => {
                tx.outbox.send(pck).await?;
            },
        };
        Ok(())
    }

    pub async fn send_reply_msg<M>(&mut self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone
    {
        let pck = Packet::from(msg).to_packet_data(self.wire_format)?;
        self.send_reply(pck).await
    }

    #[cfg(feature="enable_server")]
    pub async fn send_others(&mut self, pck: PacketData) -> Result<(), CommsError> {
        match &mut self.direction {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(tx) => {
                tx.send_others(pck).await?;
            },
            _ => { }
        };
        Ok(())
    }

    pub async fn send_all(&mut self, pck: PacketData) -> Result<(), CommsError> {
        match &mut self.direction {
            #[cfg(feature="enable_server")]
            TxDirection::Downcast(tx) => {
                tx.send_all(pck).await?;
            },
            TxDirection::Upcast(tx) => {
                tx.outbox.send(pck).await?;
            },
        };
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
}

impl Drop
for Tx
{
    fn drop(&mut self)
    {
        #[cfg(feature = "enable_super_verbose")]
        trace!("drop(node-tx)");
    }
}

#[derive(Debug)]
pub(crate) struct TxGroupSpecific
{
    pub me_id: u64,
    pub me_tx: Arc<Mutex<Upstream>>,
    pub group: Arc<Mutex<TxGroup>>,
}

impl TxGroupSpecific
{
    #[cfg(feature="enable_server")]
    pub async fn send_reply(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let mut tx = self.me_tx.lock().await;
        tx.outbox.send(pck).await?;
        Ok(())
    }

    #[cfg(feature="enable_server")]
    pub async fn send_others(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let mut group = self.group.lock().await;
        group.send(pck, Some(self.me_id)).await?;
        Ok(())
    }

    #[cfg(feature="enable_server")]
    pub async fn send_all(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let mut group = self.group.lock().await;
        group.send(pck, None).await?;
        Ok(())
    }

    #[cfg(feature="enable_server")]
    pub(crate) fn replace_group(&mut self, group: Arc<Mutex<TxGroup>>) -> Arc<Mutex<TxGroup>>
    {
        std::mem::replace(&mut self.group, group)
    }
}

#[derive(Debug, Default)]
pub(crate) struct TxGroup
{
    pub all: FxHashMap<u64, Weak<Mutex<Upstream>>>,
}

impl TxGroup
{
    #[cfg(feature="enable_server")]
    pub(crate) async fn send(&mut self, pck: PacketData, skip: Option<u64>) -> Result<(), CommsError>
    {
        match self.all.len() {
            1 => {
                if let Some(tx) = self.all.values().next().iter().filter_map(|a| Weak::upgrade(a)).next() {
                    let mut tx = tx.lock().await;
                    error!("{}, {}", tx.id, skip);
                    if Some(tx.id) != skip {
                        tx.outbox.send(pck).await?;
                    }
                }
            },
            _ => {
                let all = self.all.values().filter_map(|a| Weak::upgrade(a));
                for tx in all {
                    let mut tx = tx.lock().await;
                    error!("{}, {}", tx.id, skip);
                    if Some(tx.id) != skip {
                        tx.outbox.send(pck.clone()).await?;
                    }
                }
            }
        }
        Ok(())
    }
}