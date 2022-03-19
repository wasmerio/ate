use async_trait::async_trait;
use error_chain::bail;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_futures::{Instrument, WithSubscriber};

use super::client::MeshClient;
use super::core::*;
use super::msg::*;
use super::server::SessionContext;
use super::MeshSession;
use super::Registry;
use super::*;
use crate::chain::*;
use crate::comms::ServerProcessorFascade;
use crate::comms::TxDirection;
use crate::comms::TxGroup;
use crate::comms::*;
use crate::conf::*;
use crate::crypto::AteHash;
use crate::engine::TaskEngine;
use crate::error::*;
use crate::flow::OpenAction;
use crate::flow::OpenFlow;
use crate::index::*;
use crate::prelude::*;
use crate::spec::SerializationFormat;
use crate::time::ChainTimestamp;
use crate::transaction::*;
use crate::trust::*;

struct Redirect<C>
where
    C: Send + Sync + Default + 'static,
{
    tx: Tx,
    _marker1: PhantomData<C>,
}

impl<C> Drop for Redirect<C>
where
    C: Send + Sync + Default,
{
    fn drop(&mut self) {
        debug!("drop(redirect)");
    }
}

#[async_trait]
impl<C> InboxProcessor<Message, C> for Redirect<C>
where
    C: Send + Sync + Default + 'static,
{
    async fn process(&mut self, pck: PacketWithContext<Message, C>) -> Result<(), CommsError> {
        self.tx.send_reply(pck.data).await?;
        Ok(())
    }

    async fn shutdown(&mut self, addr: SocketAddr) {
        debug!("disconnected: {}", addr.to_string());
    }
}

pub(super) async fn redirect<C>(
    root: Arc<MeshRoot>,
    node_addr: MeshAddress,
    omit_data: bool,
    hello_path: &str,
    chain_key: ChainKey,
    from: ChainTimestamp,
    tx: Tx,
    exit: broadcast::Receiver<()>,
) -> Result<Tx, CommsError>
where
    C: Send + Sync + Default + 'static,
{
    let metrics = Arc::clone(&tx.metrics);
    let throttle = Arc::clone(&tx.throttle);
    let fascade = Redirect {
        tx,
        _marker1: PhantomData::<C>,
    };

    debug!("redirect to {}", node_addr);

    // Build a configuration that forces connecting to a specific ndoe
    let mut conf = root.cfg_mesh.clone();
    conf.force_connect = Some(node_addr.clone());
    if let Some(cert) = &root.cfg_mesh.listen_certificate {
        conf.certificate_validation = CertificateValidation::AllowedCertificates(vec![cert.hash()]);
    } else {
        conf.certificate_validation = CertificateValidation::AllowAll;
    }
    let conf = MeshConfig::new(conf).connect_to(node_addr);

    // Attempt to connect to the other machine
    let mut relay_tx = crate::comms::connect(
        &conf,
        hello_path.to_string(),
        root.server_id,
        fascade,
        metrics,
        throttle,
        exit,
    )
    .await?;

    // Send a subscribe packet to the server
    relay_tx
        .send_all_msg(Message::Subscribe {
            chain_key,
            from,
            allow_redirect: false,
            omit_data,
        })
        .await?;

    // All done
    Ok(relay_tx)
}
