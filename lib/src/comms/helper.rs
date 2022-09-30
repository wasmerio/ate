use async_trait::async_trait;
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::io::Error as TError;
use tokio::io::ErrorKind;
#[allow(unused_imports)]
use tokio::io::{self};
#[cfg(feature = "enable_full")]
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::comms::NodeId;
use crate::comms::*;
use crate::crypto::*;
use crate::error::*;
use crate::spec::*;

use super::Metrics;
use super::Packet;
use super::PacketData;
use super::PacketWithContext;
use super::StreamRx;
use super::Throttle;
use crate::conf::MeshConnectAddr;

#[async_trait]
pub(crate) trait InboxProcessor<M, C>
where
    Self: Send + Sync,
    M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
    C: Send + Sync,
{
    async fn process(&mut self, pck: PacketWithContext<M, C>) -> Result<(), CommsError>;

    async fn shutdown(&mut self, addr: MeshConnectAddr);
}

#[cfg(feature = "enable_full")]
pub(super) fn setup_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    Ok(())
}

#[allow(dead_code)]
#[allow(unused_variables)]
pub(super) async fn process_inbox<M, C>(
    mut rx: StreamRx,
    rx_proto: StreamProtocol,
    mut inbox: Box<dyn InboxProcessor<M, C>>,
    metrics: Arc<StdMutex<Metrics>>,
    throttle: Arc<StdMutex<Throttle>>,
    id: NodeId,
    peer_id: NodeId,
    sock_addr: MeshConnectAddr,
    context: Arc<C>,
    wire_format: SerializationFormat,
    wire_encryption: Option<EncryptKey>,
    mut exit: broadcast::Receiver<()>,
) -> Result<(), CommsError>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
    C: Send + Sync,
{
    let ret = async {
        // Throttling variables
        let throttle_interval = chrono::Duration::milliseconds(50);
        let mut last_throttle = chrono::offset::Utc::now();
        let mut current_received = 0u64;
        let mut current_sent = 0u64;
        let mut hickup_count = 0u32;

        // Main read loop
        loop {
            // Read the next request
            let buf = async {
                // If the throttle has triggered
                let now = chrono::offset::Utc::now();
                let delta = now - last_throttle;
                if delta > throttle_interval {
                    last_throttle = now;

                    // Compute the deltas
                    let (mut delta_received, mut delta_sent) = {
                        let metrics = metrics.lock().unwrap();
                        let delta_received = metrics.received - current_received;
                        let delta_sent = metrics.sent - current_sent;
                        current_received = metrics.received;
                        current_sent = metrics.sent;
                        (delta_received as i64, delta_sent as i64)
                    };

                    // Normalize the delta based on the time that passed
                    delta_received *= 1000i64;
                    delta_sent *= 1000i64;
                    delta_received /= delta.num_milliseconds();
                    delta_sent /= delta.num_milliseconds();

                    // We throttle the connection based off the current metrics and a calculated wait time
                    let wait_time = {
                        let throttle = throttle.lock().unwrap();
                        let wait1 = throttle
                            .download_per_second
                            .map(|limit| limit as i64)
                            .filter(|limit| delta_sent.gt(limit))
                            .map(|limit| {
                                chrono::Duration::milliseconds(
                                    ((delta_sent - limit) * 1000i64) / limit,
                                )
                            });
                        let wait2 = throttle
                            .upload_per_second
                            .map(|limit| limit as i64)
                            .filter(|limit| delta_received.gt(limit))
                            .map(|limit| {
                                chrono::Duration::milliseconds(
                                    ((delta_received - limit) * 1000i64) / limit,
                                )
                            });

                        // Whichever is the longer wait is the one we shall do
                        match (wait1, wait2) {
                            (Some(a), Some(b)) if a >= b => Some(a),
                            (Some(_), Some(b)) => Some(b),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        }
                    };

                    // We wait outside the throttle lock otherwise we will break things
                    if let Some(wait_time) = wait_time {
                        if let Ok(wait_time) = wait_time.to_std() {
                            trace!("trottle wait: {}ms", wait_time.as_millis());
                            crate::engine::sleep(wait_time).await;
                        }
                    }
                }

                rx.read().await
            };
            let buf = {
                select! {
                    _ = exit.recv() => {
                        debug!("received exit broadcast - {} - id={} peer={}", sock_addr, id.to_short_string().as_str(), peer_id.to_short_string().as_str());
                        break;
                    },
                    a = buf => a
                }
            }?;

            // Update the metrics with all this received data
            {
                let mut metrics = metrics.lock().unwrap();
                metrics.received += buf.len() as u64;
                metrics.requests += 1u64;
            }

            // Deserialize it
            let msg: M = wire_format.deserialize_ref(&buf)
                .map_err(SerializationError::from)?;
            let pck = Packet { msg };

            // Process it
            let pck = PacketWithContext {
                data: PacketData {
                    bytes: Bytes::from(buf),
                    wire_format,
                },
                context: Arc::clone(&context),
                packet: pck,
                id,
                peer_id,
            };

            // Its time to process the packet
            let rcv = inbox.process(pck);
            match rcv.await {
                Ok(a) => {
                    if hickup_count > 0 {
                        debug!("inbox-recovered: recovered from hickups {}", hickup_count);
                    }
                    hickup_count = 0;
                    a
                }
                Err(CommsError(CommsErrorKind::Disconnected, _)) => {
                    break;
                }
                Err(CommsError(CommsErrorKind::IO(err), _))
                    if err.kind() == std::io::ErrorKind::BrokenPipe =>
                {
                    if rx_proto.is_web_socket() && hickup_count < 10 {
                        hickup_count += 1;
                        continue;
                    }
                    debug!("inbox-debug: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::IO(err), _))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    debug!("inbox-debug: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::IO(err), _))
                    if err.kind() == std::io::ErrorKind::ConnectionAborted =>
                {
                    warn!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::IO(err), _))
                    if err.kind() == std::io::ErrorKind::ConnectionReset =>
                {
                    warn!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::ReadOnly, _)) => {
                    continue;
                }
                Err(CommsError(CommsErrorKind::NotYetSubscribed, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::NotYetSubscribed);
                    break;
                }
                Err(CommsError(CommsErrorKind::CertificateTooWeak(needed, actual), _)) => {
                    error!(
                        "inbox-err: {}",
                        CommsErrorKind::CertificateTooWeak(needed, actual)
                    );
                    break;
                }
                Err(CommsError(CommsErrorKind::MissingCertificate, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::MissingCertificate);
                    break;
                }
                Err(CommsError(CommsErrorKind::ServerCertificateValidation, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::ServerCertificateValidation);
                    break;
                }
                Err(CommsError(CommsErrorKind::ServerEncryptionWeak, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::ServerEncryptionWeak);
                    break;
                }
                Err(CommsError(CommsErrorKind::FatalError(err), _)) => {
                    error!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::SendError(err), _)) => {
                    warn!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(
                    CommsErrorKind::ValidationError(ValidationErrorKind::Many(errs)),
                    _,
                )) => {
                    for err in errs.iter() {
                        trace!("val-err: {}", err);
                    }

                    #[cfg(debug_assertions)]
                    warn!("inbox-debug: {} validation errors", errs.len());
                    #[cfg(not(debug_assertions))]
                    debug!("inbox-debug: {} validation errors", errs.len());
                    continue;
                }
                Err(CommsError(CommsErrorKind::ValidationError(err), _)) => {
                    #[cfg(debug_assertions)]
                    warn!("inbox-debug: validation error - {}", err);
                    #[cfg(not(debug_assertions))]
                    debug!("inbox-debug: validation error - {}", err);
                    continue;
                }
                Err(err) => {
                    warn!("inbox-error: {}", err.to_string());
                    continue;
                }
            }
        }
        Ok(())
    }
    .await;

    inbox.shutdown(sock_addr).await;
    ret
}
