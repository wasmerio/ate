#![allow(unused_imports)]
#[cfg(feature = "enable_server")]
use super::Listener;
use super::MeshConfig;
use crate::comms::Metrics;
use crate::comms::NodeId;
use crate::comms::PacketData;
use crate::comms::PacketWithContext;
#[cfg(feature = "enable_server")]
use crate::comms::ServerProcessor;
use crate::comms::Throttle;
use crate::comms::Tx;
use crate::crypto::{EncryptKey, InitializationVector, PrivateEncryptKey, PublicEncryptKey};
use crate::engine::TaskEngine;
use crate::error::*;
use crate::prelude::*;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum TestMessage {
    Noop,
    Rejected(Box<TestMessage>),
    Ping(String),
    Pong(String),
}

#[cfg(test)]
impl Default for TestMessage {
    fn default() -> TestMessage {
        TestMessage::Noop
    }
}

#[derive(Default)]
struct DummyContext {}

#[cfg(all(feature = "enable_server", feature = "enable_client"))]
#[tokio::main(flavor = "current_thread")]
#[test]
async fn test_server_client_for_comms_with_tcp() -> Result<(), AteError> {
    test_server_client_for_comms(StreamProtocol::Tcp, 4001).await
}

#[cfg(all(feature = "enable_server", feature = "enable_client"))]
#[tokio::main(flavor = "current_thread")]
#[test]
async fn test_server_client_for_comms_with_websocket() -> Result<(), AteError> {
    test_server_client_for_comms(StreamProtocol::WebSocket, 4011).await
}

#[cfg(test)]
pub(crate) fn mock_test_mesh(port: u16) -> ConfMesh {
    let mut roots = Vec::new();
    #[cfg(feature = "enable_dns")]
    roots.push(MeshAddress::new(
        IpAddr::from_str("127.0.0.1").unwrap(),
        port,
    ));
    #[cfg(not(feature = "enable_dns"))]
    roots.push(MeshAddress::new("localhost", port));

    let remote = url::Url::parse(format!("{}://localhost", Registry::guess_schema(port)).as_str()).unwrap();
    let ret = ConfMesh::new("localhost", remote, roots.iter());
    ret
}

#[cfg(all(feature = "enable_server", feature = "enable_client"))]
#[cfg(test)]
async fn test_server_client_for_comms(
    wire_protocol: StreamProtocol,
    port: u16,
) -> Result<(), AteError> {
    use crate::comms::helper::InboxProcessor;

    TaskEngine::run_until(async move {
        crate::utils::bootstrap_test_env();

        let listener;
        let wire_format = SerializationFormat::MessagePack;
        let cert = PrivateEncryptKey::generate(KeySize::Bit192);
        {
            // Start the server
            info!("starting listen server on 127.0.0.1");

            let mut cfg = mock_test_mesh(port);
            cfg.wire_protocol = wire_protocol;
            cfg.wire_format = wire_format;
            cfg.wire_encryption = Some(KeySize::Bit192);
            let cfg = MeshConfig::new(cfg)
                .listen_on(IpAddr::from_str("127.0.0.1").unwrap(), port)
                .listen_cert(cert.clone());

            #[derive(Debug, Clone, Default)]
            struct Handler {}
            #[async_trait]
            impl ServerProcessor<TestMessage, DummyContext> for Handler {
                async fn process(
                    &'_ self,
                    pck: PacketWithContext<TestMessage, DummyContext>,
                    tx: &'_ mut Tx,
                ) -> Result<(), CommsError> {
                    let pck: super::Packet<TestMessage> = pck.packet;
                    match &pck.msg {
                        TestMessage::Ping(txt) => {
                            tx.send_reply_msg(TestMessage::Pong(txt.clone()))
                                .await
                                .unwrap();
                        }
                        _ => {}
                    };
                    Ok(())
                }
                async fn shutdown(&self, _addr: SocketAddr) {}
            }

            let (exit_tx, _exit_rx) = broadcast::channel(1);
            let server_id = NodeId::generate_server_id(0);
            listener =
                Listener::new(&cfg, server_id, Arc::new(Handler::default()), exit_tx).await?;
            {
                let mut guard = listener.lock().unwrap();
                guard.add_route("/comm-test")?;
            };
        };

        #[cfg(feature = "enable_dns")]
        {
            // Start the client
            info!("start another client that will connect to the server");

            #[derive(Debug, Clone, Default)]
            struct Handler {}
            #[async_trait]
            impl InboxProcessor<TestMessage, ()> for Handler {
                async fn process(
                    &mut self,
                    pck: PacketWithContext<TestMessage, ()>,
                ) -> Result<(), CommsError> {
                    let pck: super::Packet<TestMessage> = pck.packet;
                    if let TestMessage::Pong(txt) = pck.msg {
                        assert_eq!("hello", txt.as_str());
                    } else {
                        panic!("Wrong message type returned")
                    }
                    Ok(())
                }
                async fn shutdown(&mut self, _addr: SocketAddr) {}
            }
            let inbox = Handler::default();
            let client_id = NodeId::generate_client_id();
            let metrics = Arc::new(StdMutex::new(Metrics::default()));
            let throttle = Arc::new(StdMutex::new(Throttle::default()));

            let (_exit_tx, exit_rx) = broadcast::channel(1);
            let mut cfg = mock_test_mesh(port);
            cfg.wire_protocol = wire_protocol;
            cfg.wire_format = wire_format;
            cfg.wire_encryption = Some(KeySize::Bit192);
            cfg.certificate_validation =
                CertificateValidation::AllowedCertificates(vec![cert.hash()]);
            let cfg = MeshConfig::new(cfg).connect_to(MeshAddress {
                host: IpAddr::from_str("127.0.0.1").unwrap(),
                port,
            });
            let mut client_tx = super::connect(
                &cfg,
                "/comm-test".to_string(),
                client_id,
                inbox,
                metrics,
                throttle,
                exit_rx,
            )
            .await?;

            // We need to test it alot
            info!("send lots of hellos");
            for _n in 0..1000 {
                // Send a ping
                let test = "hello".to_string();
                client_tx
                    .send_reply_msg(TestMessage::Ping(test.clone()))
                    .await
                    .unwrap();
            }
        }
        Ok(())
    })
    .await
}
