use async_trait::async_trait;
use ate::comms::{Stream, StreamProtocol};
use ate::mesh::MeshRoot;
use std::net::SocketAddr;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use hyper::upgrade::Upgraded;
use hyper_tungstenite::WebSocketStream;

use super::server::ServerCallback;

pub struct ServerMeshAdapter {
    root: Arc<MeshRoot>,
}

impl ServerMeshAdapter {
    pub fn new(root: &Arc<MeshRoot>) -> Self {
        ServerMeshAdapter {
            root: Arc::clone(root),
        }
    }
}

#[async_trait]
impl ServerCallback for ServerMeshAdapter {
    async fn web_socket(
        &self,
        ws: WebSocketStream<Upgraded>,
        sock_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create the stream object
        let stream = Stream::HyperWebSocket(ws, StreamProtocol::WebSocket);
        self.root.accept_stream(stream, sock_addr).await?;
        Ok(())
    }
}
