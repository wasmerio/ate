#![allow(unused_imports)]
use log::{info, warn, debug};

use tokio::sync::mpsc;

use crate::error::*;
use std::sync::Arc;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bytes::Bytes;
use crate::spec::*;

pub(crate) trait BroadcastContext
{
    fn broadcast_group(&self) -> Option<u64>;
}

#[derive(Debug, Clone)]
pub(crate) struct PacketData
{
    pub bytes: Bytes,
    pub reply_here: Option<mpsc::Sender<PacketData>>,
    pub skip_here: Option<u64>,
    pub wire_format: SerializationFormat,
}

#[derive(Debug, Clone)]
pub(crate) struct BroadcastPacketData
{
    pub group: Option<u64>,
    pub data: PacketData,   
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
    pub(crate) async fn reply(&self, msg: M) -> Result<(), CommsError> {
        if self.data.reply_here.is_none() { return Ok(()); }
        Ok(Self::reply_at(self.data.reply_here.as_ref(), self.data.wire_format, msg).await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at(at: Option<&mpsc::Sender<PacketData>>, format: SerializationFormat, msg: M) -> Result<(), CommsError> {
        Ok(PacketData::reply_at(at, format, msg).await?)
    }
}

impl PacketData
{
    #[allow(dead_code)]
    pub(crate) async fn reply<M>(&self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        if self.reply_here.is_none() { return Ok(()); }
        Ok(
            Self::reply_at(self.reply_here.as_ref(), self.wire_format, msg).await?
        )
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at<M>(at: Option<&mpsc::Sender<PacketData>>, wire_format: SerializationFormat, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        if at.is_none() { return Ok(()); }

        let pck = PacketData {
            bytes: Bytes::from(wire_format.serialize(&msg)?),
            reply_here: None,
            skip_here: None,
            wire_format,
        };

        if let Some(tx) = at {
            tx.send(pck).await?;
        } else {
            return Err(CommsError::NoReplyChannel);
        }

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
                reply_here: None,
                skip_here: None,
                wire_format,
            }
        )
    }
}