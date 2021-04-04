#[allow(unused_imports)]
use log::{info, warn, debug};
use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use tokio::sync::mpsc;
use std::{marker::PhantomData};
use tokio::sync::broadcast;
use std::sync::Arc;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, de::DeserializeOwned};

use crate::error::*;
use crate::spec::*;

use super::conf::Upstream;
use super::conf::NodeState;
use super::Packet;
use super::PacketData;
use super::PacketWithContext;

#[derive(Debug)]
pub(crate) struct NodeTx<C>
where C: Send + Sync
{
    pub downcast: Arc<broadcast::Sender<PacketData>>,
    pub upcast: FxHashMap<u64, Upstream>,
    pub state: Arc<StdMutex<NodeState>>,
    pub wire_format: SerializationFormat,
    pub _marker: PhantomData<C>,
}

pub(crate) struct NodeRx<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync
{
    pub rx: mpsc::Receiver<PacketWithContext<M, C>>,
    pub state: Arc<StdMutex<NodeState>>,
    pub _marker: PhantomData<C>,
}

#[allow(dead_code)]
impl<C> NodeTx<C>
where C: Send + Sync + Default + 'static
{
    pub(crate) async fn downcast_packet(&self, pck: PacketData) -> Result<(), CommsError> {
        self.downcast.send(pck)?;
        Ok(())
    }

    pub(crate) async fn downcast<M>(&self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
    {
        self.downcast_packet(Packet::from(msg).to_packet_data(self.wire_format)?).await
    }

    pub(crate) async fn upcast_packet(&self, pck: PacketData) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        upcast.outbox.send(pck).await?;
        Ok(())
    }

    pub(crate) async fn upcast<M>(&self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
    {
        self.upcast_packet(Packet::from(msg).to_packet_data(self.wire_format)?).await
    }

    pub(crate) async fn downcast_many(&self, pcks: Vec<PacketData>) -> Result<(), CommsError> {
        for pck in pcks {
            self.downcast.send(pck)?;
        }
        Ok(())
    }

    pub(crate) async fn upcast_many(&self, pcks: Vec<PacketData>) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        for pck in pcks {
            upcast.outbox.send(pck).await?;
        }
        Ok(())
    }

    pub(crate) fn connected(&self) -> i32 {
        let state = self.state.lock();
        state.connected
    }
}

#[allow(dead_code)]
impl<M, C> NodeRx<M, C>
where C: Send + Sync + Default + 'static,
      M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
{
    pub async fn recv(&mut self) -> Option<PacketWithContext<M, C>>
    {
        self.rx.recv().await
    }
}