#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace};

use tokio::sync::mpsc;

use crate::error::*;
use std::sync::Arc;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bytes::Bytes;
use crate::spec::*;
use crate::comms::*;

pub(crate) trait BroadcastContext
{
    fn broadcast_group(&self) -> Option<u64>;
}

#[derive(Debug, Clone)]
pub(crate) struct PacketData
{
    pub bytes: Bytes,
    pub wire_format: SerializationFormat,
}

#[derive(Debug)]
pub(crate) struct PacketWithContext<M, C>
where M: Send + Sync + Clone,
      C: Send + Sync
{
    pub packet: Packet<M>,
    pub data: PacketData,
    pub context: Arc<C>,
}

impl<M, C> PacketWithContext<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync
{
    #[allow(dead_code)]
    pub(crate) async fn reply(&self, tx: &mut StreamTxChannel, msg: M) -> Result<(), CommsError> {
        Ok(Self::reply_at(tx, self.data.wire_format, msg).await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at(tx: &mut StreamTxChannel, format: SerializationFormat, msg: M) -> Result<(), CommsError> {
        Ok(PacketData::reply_at(tx, format, msg).await?)
    }
}

impl PacketData
{
    #[allow(dead_code)]
    pub(crate) async fn reply<M>(&self, tx: &mut StreamTxChannel, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        Ok(
            Self::reply_at(tx, self.wire_format, msg).await?
        )
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at<M>(tx: &mut StreamTxChannel, wire_format: SerializationFormat, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        let pck = PacketData {
            bytes: Bytes::from(wire_format.serialize(&msg)?),
            wire_format,
        };

        tx.send(pck).await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Packet<M>
where M: Send + Sync + Clone
{
    pub msg: M,
}

impl<M> From<M>
for Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    fn from(msg: M) -> Packet<M> {
        Packet {
            msg,
        }
    }
}

impl<M> Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub(crate) fn to_packet_data(self, wire_format: SerializationFormat) -> Result<PacketData, CommsError>
    {
        let buf = wire_format.serialize(&self.msg)?;
        Ok(
            PacketData {
                bytes: Bytes::from(buf),
                wire_format,
            }
        )
    }
}