#![allow(unused_imports)]
use log::{info, warn, debug};
use crate::crypto::{EncryptKey, PrivateEncryptKey, PublicEncryptKey, InitializationVector};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::prelude::*;
use super::NodeConfig;

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum TestMessage
{
    Noop,
    Rejected(Box<TestMessage>),
    Ping(String),
    Pong(String),
}

#[cfg(test)]
impl Default
for TestMessage
{
    fn default() -> TestMessage {
        TestMessage::Noop
    }
}

#[tokio::main]
#[test]
async fn test_server_client_for_comms() {
    crate::utils::bootstrap_env();
    
    let wire_format = SerializationFormat::MessagePack;
    {
        // Start the server
        info!("starting listen server on 127.0.0.1");
        let cfg = NodeConfig::new(wire_format)
            .wire_encryption(Some(KeySize::Bit256))
            .listen_on(IpAddr::from_str("127.0.0.1")
            .unwrap(), 4001);
        let (_, mut server_rx) = super::listen::<TestMessage, ()>(&cfg).await;

        // Create a background thread that will respond to pings with pong
        info!("creating server worker thread");
        tokio::spawn(async move {
            while let Some(pck) = server_rx.recv().await {
                let data = pck.data;
                let pck: super::Packet<TestMessage> = pck.packet;
                match &pck.msg {
                    TestMessage::Ping(txt) => {
                        let _ = data.reply(TestMessage::Pong(txt.clone())).await;
                    },
                    _ => {}
                };
            }
        });
    }

    /* This has been disabled for now as we deprecated the built in relay functionality and will
     * build it again when the time is right
    {
        // Start the reply
        info!("start a client that will be relay server");
        let cfg = NodeConfig::new(wire_format)
            .wire_encryption(Some(KeySize::Bit256))
            .listen_on(IpAddr::from_str("127.0.0.1").unwrap(), 4002)
            .connect_to(IpAddr::from_str("127.0.0.1").unwrap(), 4001);
        let (relay_tx, mut relay_rx) = connect::<TestMessage, ()>(&cfg, None).await;

        // Create a background thread that will respond to pings with pong
        info!("start a client worker thread");
        tokio::spawn(async move {
            while let Some(pck) = relay_rx.recv().await {
                let data = pck.data;
                let pck = pck.packet;
                match pck.msg {
                    TestMessage::Ping(_) => relay_tx.upcast_packet(data).await.unwrap(),
                    TestMessage::Pong(_) => relay_tx.downcast_packet(data).await.unwrap(),
                    _ => data.reply(TestMessage::Rejected(Box::new(pck.msg.clone()))).await.unwrap(),
                };
            }
        });
    }
    */
    
    {
        // Start the client
        info!("start another client that will connect to the relay");
        let cfg = NodeConfig::new(wire_format)
            .wire_encryption(Some(KeySize::Bit256))
            .connect_to(IpAddr::from_str("127.0.0.1")
            .unwrap(), 4001);
        let (client_tx, mut client_rx) = super::connect::<TestMessage, ()>(&cfg, None)
            .await;

        // We need to test it alot
        info!("send lots of hellos");
        for n in 0..1000
        {
            // Send a ping
            let test = format!("hello! {}", n);
            client_tx.upcast(TestMessage::Ping(test.clone())).await.unwrap();

            // Wait for the pong
            let pong = client_rx.recv().await.unwrap();
            let pong = pong.packet;
            if let TestMessage::Pong(txt) = pong.msg {
                assert_eq!(test, txt);
            } else {
                panic!("Wrong message type returned")
            }
        }
    }
}