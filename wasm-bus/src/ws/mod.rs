#![allow(unused_imports)]
mod socket_builder;
mod web_socket;

pub(crate) use socket_builder::*;
pub(crate) use web_socket::*;

pub use socket_builder::SocketBuilder;
pub use web_socket::WebSocket;
pub use web_socket::RecvHalf;
pub use web_socket::SendHalf;

pub const WAPM_NAME: &'static str = "os";