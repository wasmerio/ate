use serde::*;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

use super::MessageProtocolApi;

/// Version of the stream protocol used to talk to Tokera services
#[repr(u16)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum MessageProtocolVersion
{
    V1 = 1,
    V2 = 2,
    V3 = 3,
}

impl Default
for MessageProtocolVersion
{
    fn default() -> Self {
        MessageProtocolVersion::V3
    }
}

impl MessageProtocolVersion
{
    pub fn min(&self, other: MessageProtocolVersion) -> MessageProtocolVersion {
        let first = *self as u16;
        let second = other as u16;
        let min = first.min(second);

        if first == min {
            *self
        } else {
            other
        }
    }

    pub fn upgrade(&self, mut proto: Box<dyn MessageProtocolApi + Send + Sync + 'static>) -> Box<dyn MessageProtocolApi + Send + Sync + 'static> {
        let rx = proto.take_rx();
        let tx = proto.take_tx();
        self.create(rx, tx)
    }

    pub fn create(&self, rx: Option<Box<dyn AsyncRead + Send + Sync + Unpin + 'static>>, tx: Option<Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>>) -> Box<dyn MessageProtocolApi + Send + Sync + 'static>
    {
        match self {
            MessageProtocolVersion::V1 => {
                Box::new(super::v1::MessageProtocol::new(rx, tx))
            }
            MessageProtocolVersion::V2 => {
                Box::new(super::v2::MessageProtocol::new(rx, tx))
            }
            MessageProtocolVersion::V3 => {
                Box::new(super::v3::MessageProtocol::new(rx, tx))
            }
        }
    }
}