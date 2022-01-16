use async_trait::async_trait;
use ate::comms::{Stream, StreamProtocol};
use std::net::SocketAddr;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use hyper::upgrade::Upgraded;
use hyper_tungstenite::WebSocketStream;

use ate::comms::StreamRouter;

use super::server::ServerCallback;

#[async_trait]
impl ServerCallback for StreamRouter {
    async fn web_socket(
        &self,
        ws: WebSocketStream<Upgraded>,
        sock_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>>
    {
        // Create the stream object
        let stream = Stream::HyperWebSocket(ws, StreamProtocol::WebSocket);

        self.accept_socket(stream, sock_addr).await?;
        Ok(())
    }
}
