use crate::common::MAX_MPSC;
use async_trait::async_trait;
use std::any::type_name;
use std::collections::HashMap;
use std::sync::Arc;
use std::ops::Deref;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::SerializationFormat;
use wasm_bus_ws::api;
use wasm_bus_ws::model;

use super::*;
use crate::api::*;

pub fn web_socket(
    connect: api::SocketBuilderConnectRequest,
    this_callback: Arc<dyn BusFeeder + Send + Sync + 'static>,
    mut client_callbacks: HashMap<String, Arc<dyn BusFeeder + Send + Sync + 'static>>,
) -> Result<(WebSocketInvoker, WebSocketSession), CallError> {
    let system = System::default();

    // Build all the callbacks
    let on_state_change = client_callbacks
        .remove(&type_name::<api::SocketBuilderConnectStateChangeCallback>().to_string());
    let on_received = client_callbacks
        .remove(&type_name::<api::SocketBuilderConnectReceiveCallback>().to_string());
    if on_state_change.is_none() || on_received.is_none() {
        return Err(CallError::MissingCallbacks);
    }
    let on_state_change = on_state_change.unwrap();
    let on_received = on_received.unwrap();

    // Construct the channels
    let (tx_keepalive, mut rx_keepalive) = mpsc::channel(1);
    let (tx_state, rx_state) = broadcast::channel::<model::SocketState>(10);
    let (tx_send, mut rx_send) = mpsc::channel::<Vec<u8>>(MAX_MPSC);

    // The web socket will be started in a background thread as it
    // is an asynchronous IO primative
    system.spawn_dedicated(move || async move {
        let mut rx_state = tx_state.subscribe();

        // Open the web socket
        let mut ws_sys = match system.web_socket(connect.url.as_str()).await {
            Ok(a) => a,
            Err(err) => {
                debug!("failed to create web socket ({}): {}", connect.url, err);
                let _ = tx_state.send(model::SocketState::Failed);
                BusFeederUtils::feed(
                    on_state_change.deref(),
                    SerializationFormat::Bincode,
                    api::SocketBuilderConnectStateChangeCallback(model::SocketState::Failed),
                );
                return;
            }
        };

        {
            let tx_state = tx_state.clone();
            let on_state_change = on_state_change.clone();
            ws_sys.set_onopen(Box::new(move || {
                debug!("websocket set_onopen()");
                let _ = tx_state.send(model::SocketState::Opened);
                BusFeederUtils::feed(
                    on_state_change.deref(),
                    SerializationFormat::Bincode,
                    api::SocketBuilderConnectStateChangeCallback(model::SocketState::Opened),
                );
            }));
        }

        {
            let tx_state = tx_state.clone();
            let on_state_change = on_state_change.clone();
            ws_sys.set_onclose(Box::new(move || {
                debug!("websocket set_onclose()");
                let _ = tx_state.send(model::SocketState::Closed);
                BusFeederUtils::feed(
                    on_state_change.deref(),
                    SerializationFormat::Bincode,
                    api::SocketBuilderConnectStateChangeCallback(model::SocketState::Closed),
                );
            }));
        }

        {
            ws_sys.set_onmessage(Box::new(move |data| {
                debug!("websocket recv {} bytes", data.len());
                BusFeederUtils::feed(
                    on_received.deref(),
                    SerializationFormat::Bincode,
                    api::SocketBuilderConnectReceiveCallback(data),
                );
            }));
        }

        // Wait for the socket ot open or for something bad to happen
        loop {
            select! {
                _ = rx_keepalive.recv() => {
                    return;
                }
                state = rx_state.recv() => {
                    match state {
                        Ok(state) => {
                            if state != model::SocketState::Opened {
                                return;
                            }
                            break;
                        }
                        Err(_) => {
                            return;
                        }
                    }
                }
            }
        }
        // The main loop does all the processing
        loop {
            select! {
                _ = rx_keepalive.recv() => {
                    break;
                }
                state = rx_state.recv() => {
                    if state != Ok(model::SocketState::Opened) {
                        break;
                    }
                }
                request = rx_send.recv() => {
                    if let Some(data) = request {
                        let data_len = data.len();

                        #[cfg(feature="async_ws")]
                        let ret = ws_sys.send(data).await;
                        #[cfg(not(feature="async_ws"))]
                        let ret = ws_sys.send(data);

                        if let Err(err) = ret {
                            debug!("error sending message: {}", err);
                        } else {
                            trace!("websocket sent {} bytes", data_len);
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        BusFeederUtils::feed(
            on_state_change.deref(),
            SerializationFormat::Bincode,
            api::SocketBuilderConnectStateChangeCallback(model::SocketState::Closed),
        );
    });

    // Return the invokers
    let invoker = WebSocketInvoker {
        ws: Some(WebSocket {
            tx_keepalive,
            this: this_callback,
            rx_state,
        }),
    };
    let session = WebSocketSession { tx_send };
    Ok((invoker, session))
}

pub struct WebSocket {
    #[allow(dead_code)]
    tx_keepalive: mpsc::Sender<()>,
    this: Arc<dyn BusFeeder + Send + Sync + 'static>,
    rx_state: broadcast::Receiver<model::SocketState>,
}

impl WebSocket {
    pub async fn run(mut self) {
        loop {
            let state = self.rx_state.recv().await;
            match state {
                Ok(model::SocketState::Opened) => {
                    debug!("confirmed websocket successfully opened");
                }
                Ok(model::SocketState::Closed) => {
                    debug!("confirmed websocket closed before it opened");
                    break;
                }
                Ok(_) => {
                    debug!("confirmed websocket failed before it opened");
                    break;
                }
                Err(_) => {
                    debug!("confirmed websocket closed by client");
                    break;
                }
            }
        }
        self.this.terminate();
    }
}

pub struct WebSocketInvoker {
    ws: Option<WebSocket>,
}

#[async_trait]
impl Invokable for WebSocketInvoker {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let ws = self.ws.take();
        if let Some(ws) = ws {
            let fut = Box::pin(ws.run());
            Ok(InvokeResult::ResponseThenWork(
                encode_response(SerializationFormat::Bincode, &())?,
                fut,
            ))
        } else {
            Err(CallError::Unknown)
        }
    }
}

pub struct WebSocketSession {
    tx_send: mpsc::Sender<Vec<u8>>,
}

impl Session for WebSocketSession {
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Box<dyn Invokable + 'static> {
        if topic == type_name::<api::WebSocketSendRequest>() {
            let data = match decode_request::<api::WebSocketSendRequest>(
                SerializationFormat::Bincode,
                request.as_ref(),
            ) {
                Ok(a) => a.data,
                Err(err) => {
                    return ErrornousInvokable::new(err);
                }
            };
            let data_len = data.len();

            let again = match self.tx_send.try_send(data) {
                Ok(_) => None,
                Err(mpsc::error::TrySendError::Closed(a)) => Some(a),
                Err(mpsc::error::TrySendError::Full(a)) => Some(a),
            };
            if let Some(data) = again {
                Box::new(DelayedSend {
                    data: Some(data),
                    tx: self.tx_send.clone(),
                })
            } else {
                ResultInvokable::new(
                    SerializationFormat::Bincode,
                    model::SendResult::Success(data_len),
                )
            }
        } else {
            ErrornousInvokable::new(CallError::InvalidTopic)
        }
    }
}

struct DelayedSend {
    data: Option<Vec<u8>>,
    tx: mpsc::Sender<Vec<u8>>,
}

#[async_trait]
impl Invokable for DelayedSend {
    async fn process(&mut self) -> Result<InvokeResult, CallError> {
        let mut size = 0usize;
        if let Some(data) = self.data.take() {
            size = data.len();
            let _ = self.tx.send(data).await;
        }
        ResultInvokable::new(SerializationFormat::Bincode, model::SendResult::Success(size))
            .process()
            .await
    }
}
