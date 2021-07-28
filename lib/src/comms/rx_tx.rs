#[allow(unused_imports)]
use log::{info, warn, debug};
use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use std::sync::Arc;
use std::sync::Weak;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;

use crate::error::*;
use crate::prelude::SerializationFormat;

use super::conf::Upstream;
use super::Packet;
use super::PacketData;

#[derive(Debug)]
pub(crate) enum TxDirection
{
    Downcast(TxGroupSpecific),
    UpcastOne(Upstream),
    UpcastMany(FxHashMap<u64, Upstream>)
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
            TxDirection::Downcast(tx) => {
                tx.send_reply(pck).await?;
            },
            TxDirection::UpcastOne(tx) => {
                tx.outbox.send(pck).await?;
            },
            TxDirection::UpcastMany(tx) => {
                let mut upcasts = tx.values_mut().collect::<Vec<_>>();
                let upcast = upcasts.choose_mut(&mut rand::thread_rng()).unwrap();
                upcast.outbox.send(pck).await?;
            }
        };
        Ok(())
    }

    pub async fn send_reply_msg<M>(&mut self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone
    {
        let pck = Packet::from(msg).to_packet_data(self.wire_format)?;
        self.send_reply(pck).await
    }

    pub async fn send_others(&mut self, pck: PacketData) -> Result<(), CommsError> {
        match &mut self.direction {
            TxDirection::Downcast(tx) => {
                tx.send_others(pck).await?;
            },
            _ => { }
        };
        Ok(())
    }

    pub(crate) async fn on_disconnect(&self) -> Result<(), CommsError> {
        Ok(())
    }
}

impl Drop
for Tx
{
    fn drop(&mut self)
    {
        #[cfg(feature = "enable_super_verbose")]
        debug!("drop(node-tx)");
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
    pub async fn send_reply(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let mut tx = self.me_tx.lock().await;
        tx.outbox.send(pck).await?;
        Ok(())
    }

    pub async fn send_others(&mut self, pck: PacketData) -> Result<(), CommsError>
    {
        let mut group = self.group.lock().await;
        group.send(pck, Some(self.me_id)).await?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct TxGroup
{
    pub all: Vec<Weak<Mutex<Upstream>>>,
}

impl TxGroup
{
    pub(crate) fn add(&mut self, tx: &Arc<Mutex<Upstream>>) {
        self.all.push(Arc::downgrade(tx));
    }

    pub(crate) async fn send(&mut self, pck: PacketData, skip: Option<u64>) -> Result<(), CommsError>
    {
        match self.all.len() {
            1 => {
                if let Some(tx) = Weak::upgrade(&self.all[0]) {
                    let mut tx = tx.lock().await;
                    if Some(tx.id) != skip {
                        tx.outbox.send(pck).await?;
                    }
                } else {
                    self.all.clear();
                }
            },
            _ => {
                let mut n = 0usize;
                while n < self.all.len() {
                    if let Some(tx) = Weak::upgrade(&self.all[n]) {
                        let mut tx = tx.lock().await;
                        if Some(tx.id) != skip {
                            tx.outbox.send(pck.clone()).await?;
                        }
                        n = n + 1;
                    } else {
                        self.all.remove(n);
                    }
                }
            }
        }
        Ok(())
    }
}