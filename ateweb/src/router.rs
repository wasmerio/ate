use async_trait::async_trait;
use ate::comms::{Stream, StreamProtocol};
use std::net::SocketAddr;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use http::*;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::WebSocketStream;
use std::result::Result;

use ate::comms::StreamRouter;

use super::server::ServerCallback;

#[async_trait]
impl ServerCallback for StreamRouter {
    async fn web_socket(
        &self,
        ws: WebSocketStream<Upgraded>,
        sock_addr: SocketAddr,
        uri: Option<http::Uri>,
        headers: Option<http::HeaderMap>
    ) -> Result<(), Box<dyn std::error::Error>>
    {
        let stream = Stream::HyperWebSocket(ws, StreamProtocol::WebSocket);
        self.accept_socket(stream, sock_addr, uri, headers).await?;
        Ok(())
    }

    async fn post_request(
        &self,
        body: Vec<u8>,
        sock_addr: SocketAddr,
        uri: http::Uri,
        headers: http::HeaderMap,
    ) -> Result<Vec<u8>, StatusCode> {
        StreamRouter::post_request(self, body, sock_addr, uri, headers).await
    }
}
