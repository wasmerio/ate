#[allow(unused_imports)]
use log::{info, warn, debug};

mod packet;
mod hello;
mod key_exchange;
mod conf;
mod helper;
mod server;
mod client;
mod rx_tx;
mod test;

pub(crate) use packet::Packet;
pub(crate) use packet::PacketData;
pub(crate) use packet::BroadcastPacketData;
pub(crate) use packet::PacketWithContext;
pub(crate) use packet::BroadcastContext;
pub(crate) use conf::NodeConfig;

pub(crate) use rx_tx::NodeRx;
pub(crate) use rx_tx::NodeTx;
pub(crate) use rx_tx::TxDirection;

pub(crate) use server::listen;
pub(crate) use client::connect;

#[cfg(not(feature="websockets"))]
mod stream_impl
{
    pub type Stream = tokio::net::TcpStream;
    pub type StreamRx = tokio::net::tcp::OwnedReadHalf;
    pub type StreamTx = tokio::net::tcp::OwnedWriteHalf;
}
#[cfg(feature="websockets")]
mod stream_impl
{
    pub type Stream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;
}

pub use stream_impl::*;