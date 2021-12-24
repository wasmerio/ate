use crate::common::MAX_MPSC;
use async_trait::async_trait;
use std::any::type_name;
use std::collections::HashMap;
use tokio::select;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::backend::ws::*;

use super::*;
use crate::api::*;

pub fn web_socket(
    connect: Connect,
    mut client_callbacks: HashMap<String, WasmBusCallback>,
) -> Result<(WebSocketInvoker, WebSocketSession), CallError> {
    let system = System::default();

    // Build all the callbacks
    let on_state_change = client_callbacks.remove(&type_name::<SocketState>().to_string());
    let on_received = client_callbacks.remove(&type_name::<Received>().to_string());
    if on_state_change.is_none() || on_received.is_none() {
        return Err(CallError::MissingCallbacks);
    }
    let on_state_change = on_state_change.unwrap();
    let on_state_change_waker = on_state_change.waker();
    let on_received = on_received.unwrap();
    let on_received_waker = on_received.waker();

    // Construct the channels
    let (tx_keepalive, mut rx_keepalive) = mpsc::channel(1);
    let (tx_recv, rx_recv) = mpsc::channel(MAX_MPSC);
    let (tx_state, rx_state) = mpsc::channel::<SocketState>(MAX_MPSC);
    let (tx_send, mut rx_send) = mpsc::channel::<Send>(MAX_MPSC);

    // The web socket will be started in a background thread as it
    // is an asynchronous IO primative
    system.spawn_dedicated(move || async move {
        // Open the web socket
        let mut ws_sys = match system.web_socket(connect.url.as_str()).await {
            Ok(a) => a,
            Err(err) => {
                debug!("failed to create web socket ({}): {}", connect.url, err);
                let _ = tx_state.send(SocketState::Failed).await;
                return;
            }
        };

        // The inner state is used to chain then states so the web socket
        // properly starts and exits when it should
        let (tx_state_inner, mut rx_state_inner) = mpsc::channel::<SocketState>(MAX_MPSC);

        {
            let tx_state_inner = tx_state_inner.clone();
            ws_sys.set_onopen(Box::new(move || {
                let again = match tx_state_inner.try_send(SocketState::Opened) {
                    Ok(_) => None,
                    Err(mpsc::error::TrySendError::Closed(a)) => Some(a),
                    Err(mpsc::error::TrySendError::Full(a)) => Some(a),
                };
                if let Some(data) = again {
                    let tx_state_inner = tx_state_inner.clone();
                    system.fork_shared(move || async move {
                        let _ = tx_state_inner.send(data).await;
                    });
                }
            }));
        }

        {
            let tx_state_inner = tx_state_inner.clone();
            ws_sys.set_onclose(Box::new(move || {
                let again = match tx_state_inner.try_send(SocketState::Closed) {
                    Ok(_) => None,
                    Err(mpsc::error::TrySendError::Closed(a)) => Some(a),
                    Err(mpsc::error::TrySendError::Full(a)) => Some(a),
                };
                if let Some(data) = again {
                    let tx_state_inner = tx_state_inner.clone();
                    system.fork_shared(move || async move {
                        let _ = tx_state_inner.send(data).await;
                    });
                }
            }));
        }

        {
            let tx_recv = tx_recv.clone();
            ws_sys.set_onmessage(Box::new(move |data| {
                debug!("websocket recv {} bytes", data.len());
                let again = match tx_recv.try_send(data) {
                    Ok(_) => None,
                    Err(mpsc::error::TrySendError::Closed(a)) => Some(a),
                    Err(mpsc::error::TrySendError::Full(a)) => Some(a),
                };
                if let Some(data) = again {
                    let tx_recv = tx_recv.clone();
                    system.fork_shared(move || async move {
                        if let Err(err) = tx_recv.send(data).await {
                            trace!("websocket bytes silently dropped - {}", err);
                        }
                    });
                }
            }));
        }

        // Wait for the socket ot open or for something bad to happen
        {
            let on_state_change_waker = on_state_change_waker.clone();
            if let Some(state) = rx_state_inner.recv().await {
                let _ = tx_state.send(state.clone()).await;
                on_state_change_waker.wake();
                if state != SocketState::Opened {
                    return;
                }
            }
        }

        // The main loop does all the processing
        loop {
            select! {
                _ = rx_keepalive.recv() => {
                    on_state_change_waker.wake();
                    return;
                }
                state = rx_state_inner.recv() => {
                    on_state_change_waker.wake();
                    if let Some(state) = &state {
                        let _ = tx_state.send(state.clone()).await;
                    }
                    if state != Some(SocketState::Opened) {
                        return;
                    }
                }
                request = rx_send.recv() => {
                    on_received_waker.wake();
                    if let Some(request) = request {
                        let data = request.data;
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
                    }
                }
            }
        }
    });

    // Return the invokers
    let invoker = WebSocketInvoker {
        ws: Some(WebSocket {
            tx_keepalive,
            rx_state,
            rx_recv,
            on_state_change,
            on_received,
        }),
    };
    let session = WebSocketSession { tx_send };
    Ok((invoker, session))
}

pub struct WebSocket {
    #[allow(dead_code)]
    tx_keepalive: mpsc::Sender<()>,
    rx_state: mpsc::Receiver<SocketState>,
    rx_recv: mpsc::Receiver<Vec<u8>>,
    on_state_change: WasmBusCallback,
    on_received: WasmBusCallback,
}

impl WebSocket {
    pub async fn run(mut self) {
        loop {
            select! {
                state = self.rx_state.recv() => {
                    if let Some(state) = &state {
                        self.on_state_change.feed(state.clone());
                    }
                    match state {
                        Some(SocketState::Opened) => {
                            debug!("confirmed websocket successfully opened");
                        }
                        Some(SocketState::Closed) => {
                            debug!("confirmed websocket closed before it opened");
                            return;
                        }
                        Some(_) => {
                            debug!("confirmed websocket failed before it opened");
                            return;
                        }
                        None => {
                            debug!("confirmed websocket closed by client");
                            return;
                        }
                    }
                }
                data = self.rx_recv.recv() => {
                    if let Some(data) = data {
                        let received = Received {
                            data
                        };
                        self.on_received.feed(received);
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

pub struct WebSocketInvoker {
    ws: Option<WebSocket>,
}

#[async_trait]
impl Invokable for WebSocketInvoker {
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        let ws = self.ws.take();
        if let Some(ws) = ws {
            let ret = ws.run().await;
            Ok(encode_response(&ret)?)
        } else {
            Err(CallError::Unknown)
        }
    }
}

pub struct WebSocketSession {
    tx_send: mpsc::Sender<Send>,
}

impl Session for WebSocketSession {
    fn call(&mut self, topic: &str, request: Vec<u8>) -> Box<dyn Invokable + 'static> {
        if topic == type_name::<Send>() {
            let request: Send = match decode_request(request.as_ref()) {
                Ok(a) => a,
                Err(err) => {
                    return ErrornousInvokable::new(err);
                }
            };

            let data_len = request.data.len();

            let again = match self.tx_send.try_send(request) {
                Ok(_) => None,
                Err(mpsc::error::TrySendError::Closed(a)) => Some(a),
                Err(mpsc::error::TrySendError::Full(a)) => Some(a),
            };
            if let Some(request) = again {
                Box::new(DelayedSend {
                    data_len,
                    msg: Some(request),
                    tx: self.tx_send.clone(),
                })
            } else {
                ResultInvokable::new(SendResult::Success(data_len))
            }
        } else {
            ErrornousInvokable::new(CallError::InvalidTopic)
        }
    }
}

struct DelayedSend {
    data_len: usize,
    msg: Option<Send>,
    tx: mpsc::Sender<Send>,
}

#[async_trait]
impl Invokable for DelayedSend {
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        if let Some(msg) = self.msg.take() {
            let _ = self.tx.send(msg).await;
        }
        ResultInvokable::new(SendResult::Success(self.data_len))
            .process()
            .await
    }
}
