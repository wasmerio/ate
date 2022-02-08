#![allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use tokio::sync::mpsc;

use crate::comms::*;
use crate::error::*;
use crate::spec::*;
use bytes::Bytes;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PacketData {
    pub bytes: Bytes,
    pub wire_format: SerializationFormat,
}

#[derive(Debug)]
pub(crate) struct PacketWithContext<M, C>
where
    M: Send + Sync + Clone,
    C: Send + Sync,
{
    pub packet: Packet<M>,
    pub data: PacketData,
    #[allow(dead_code)]
    pub context: Arc<C>,
    #[allow(dead_code)]
    pub id: NodeId,
    #[allow(dead_code)]
    pub peer_id: NodeId,
}

impl<M, C> PacketWithContext<M, C>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone,
    C: Send + Sync,
{
    #[allow(dead_code)]
    pub(crate) async fn reply(&self, tx: &mut StreamTxChannel, msg: M) -> Result<(), CommsError> {
        Ok(Self::reply_at(tx, self.data.wire_format, msg).await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at(
        tx: &mut StreamTxChannel,
        format: SerializationFormat,
        msg: M,
    ) -> Result<(), CommsError> {
        Ok(PacketData::reply_at(tx, format, msg).await?)
    }
}

impl PacketData {
    #[allow(dead_code)]
    pub(crate) async fn reply<M>(&self, tx: &mut StreamTxChannel, msg: M) -> Result<(), CommsError>
    where
        M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        Ok(Self::reply_at(tx, self.wire_format, msg).await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at<M>(
        tx: &mut StreamTxChannel,
        wire_format: SerializationFormat,
        msg: M,
    ) -> Result<(), CommsError>
    where
        M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        let pck = PacketData {
            bytes: Bytes::from(wire_format.serialize(&msg)?),
            wire_format,
        };

        tx.send(&pck.bytes[..]).await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Packet<M>
where
    M: Send + Sync + Clone,
{
    pub msg: M,
}

impl<M> From<M> for Packet<M>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    fn from(msg: M) -> Packet<M> {
        Packet { msg }
    }
}

impl<M> Packet<M>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    pub(crate) fn to_packet_data(
        self,
        wire_format: SerializationFormat,
    ) -> Result<PacketData, CommsError> {
        let buf = wire_format.serialize(&self.msg)?;
        Ok(PacketData {
            bytes: Bytes::from(buf),
            wire_format,
        })
    }
}
