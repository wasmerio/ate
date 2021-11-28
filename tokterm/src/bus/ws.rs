use crate::common::MAX_MPSC;
use async_trait::async_trait;
use std::any::type_name;
use std::collections::HashMap;
use tokio::select;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bus::abi::CallError;
use wasm_bus::backend::ws::*;
use web_sys::{MessageEvent, WebSocket as WebSocketSys};

use crate::api::System;
use super::*;

struct WebSocketCreate {
    connect: Connect,
    result: mpsc::Sender<(WebSocketInvoker, WebSocketSession)>,
    on_state_change: WasmBusFeeder,
    on_received: WasmBusFeeder,
}

#[derive(Debug, Clone)]
pub struct WebSocketFactory {
    system: System,
    maker: mpsc::Sender<WebSocketCreate>,
}

impl WebSocketFactory {
    pub fn new() -> WebSocketFactory {
        let system = System::default();
        let (tx_factory, mut rx_factory) = mpsc::channel::<WebSocketCreate>(MAX_MPSC);
        system.spawn_local(async move {
            while let Some(create) = rx_factory.recv().await {
                // Construct the channels
                let (tx_recv, rx_recv) = mpsc::channel(MAX_MPSC);
                let (tx_send, rx_send) = mpsc::channel::<Send>(MAX_MPSC);
                let (tx_state, rx_state) = mpsc::channel::<SocketState>(MAX_MPSC);

                // Open the web socket
                let ws_sys = match WebSocketSys::new(create.connect.url.as_str()) {
                    Ok(a) => a,
                    Err(err) => {
                        debug!(
                            "failed to create web socket ({}): {:?}",
                            create.connect.url, err
                        );
                        let _ = tx_state.blocking_send(SocketState::Failed);
                        return;
                    }
                };

                let onopen_callback = {
                    let url = create.connect.url.clone();
                    let ws_sys = ws_sys.clone();
                    let tx_state = tx_state.clone();
                    let mut rx_send = Some(rx_send);
                    Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                        let _ = tx_state.blocking_send(SocketState::Opened);
                        if let Some(mut rx_send) = rx_send.take() {
                            let url = url.clone();
                            let ws_sys = ws_sys.clone();
                            system.spawn_local(async move {
                                while let Some(request) = rx_send.recv().await {
                                    let data = request.data;
                                    let data_len = data.len();
                                    let array =
                                        js_sys::Uint8Array::new_with_length(data_len as u32);
                                    array.copy_from(&data[..]);
                                    if let Err(err) = ws_sys.send_with_array_buffer(&array.buffer())
                                    {
                                        debug!("error sending message: {:?}", err);
                                    } else {
                                        trace!("websocket sent {} bytes", data_len);
                                    }
                                }
                                debug!("web socket closed by client ({})", url);
                            });
                        }
                    })
                        as Box<dyn FnMut(web_sys::ProgressEvent)>)
                };
                ws_sys.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
                onopen_callback.forget();

                let onclose_callback = {
                    let tx_state = tx_state.clone();
                    Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                        let _ = tx_state.blocking_send(SocketState::Closed);
                    })
                        as Box<dyn FnMut(web_sys::ProgressEvent)>)
                };
                ws_sys.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
                onclose_callback.forget();

                let fr = web_sys::FileReader::new().unwrap();
                let fr_c = fr.clone();
                let onloadend_cb = {
                    let tx = tx_recv.clone();
                    Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                        let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                        let len = array.byte_length() as usize;
                        trace!("websocket recv {} bytes (web_sys::Blob)", len);
                        if let Err(err) = tx.blocking_send(array.to_vec()) {
                            debug!("websocket bytes silently dropped - {}", err);
                        }
                    })
                        as Box<dyn FnMut(web_sys::ProgressEvent)>)
                };
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                onloadend_cb.forget();

                // Attach the message process
                let onmessage_callback = {
                    let tx = tx_recv.clone();
                    Closure::wrap(Box::new(move |e: MessageEvent| {
                        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                            let data = js_sys::Uint8Array::new(&abuf).to_vec();
                            debug!(
                                "websocket recv {} bytes (via js_sys::ArrayBuffer)",
                                data.len()
                            );
                            if let Err(err) = tx.blocking_send(data) {
                                trace!("websocket bytes silently dropped - {}", err);
                            }
                        } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                            fr.read_as_array_buffer(&blob).expect("blob not readable");
                        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            debug!("message event, received Text: {:?}", txt);
                        } else {
                            debug!("websocket received unknown message type");
                        }
                    }) as Box<dyn FnMut(MessageEvent)>)
                };
                ws_sys.set_binary_type(web_sys::BinaryType::Arraybuffer);
                ws_sys.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                onmessage_callback.forget();

                // Return the invokers
                let invoker = WebSocketInvoker {
                    ws: Some(WebSocket {
                        rx_state,
                        rx_recv,
                        on_state_change: create.on_state_change,
                        on_received: create.on_received,
                    }),
                };
                let session = WebSocketSession { tx_send };
                let _ = create.result.send((invoker, session)).await;
            }
        });

        WebSocketFactory {
            system,
            maker: tx_factory
        }
    }

    pub fn create(
        &self,
        request: Connect,
        mut client_callbacks: HashMap<String, WasmBusFeeder>,
    ) -> Result<(WebSocketInvoker, WebSocketSession), CallError> {
        let on_state_change = client_callbacks.remove(&type_name::<SocketState>().to_string());
        let on_received = client_callbacks.remove(&type_name::<Received>().to_string());
        if on_state_change.is_none() || on_received.is_none() {
            return Err(CallError::MissingCallbacks);
        }
        let on_state_change = on_state_change.unwrap();
        let on_received = on_received.unwrap();

        let (tx_result, mut rx_result) = mpsc::channel(1);
        let create = WebSocketCreate {
            connect: request,
            result: tx_result,
            on_state_change,
            on_received,
        };

        let _ = self.maker.blocking_send(create);

        rx_result.blocking_recv().ok_or_else(|| CallError::Aborted)
    }
}

pub struct WebSocket {
    rx_state: mpsc::Receiver<SocketState>,
    rx_recv: mpsc::Receiver<Vec<u8>>,
    on_state_change: WasmBusFeeder,
    on_received: WasmBusFeeder,
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
                        _ => {
                            debug!("confirmed websocket failed before it opened");
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
    fn call(&mut self, topic: &str, request: &Vec<u8>) -> Box<dyn Invokable + 'static> {
        if topic == type_name::<Send>() {
            let request: Send = match decode_request(request.as_ref()) {
                Ok(a) => a,
                Err(err) => {
                    return ErrornousInvokable::new(err);
                }
            };
            let data_len = request.data.len();
            let tx_send = self.tx_send.clone();
            let _ = tx_send.blocking_send(request);
            ResultInvokable::new(SendResult::Success(data_len))
            /*
            let data = &request.data;
            let data_len = data.len();
            let array = js_sys::Uint8Array::new_with_length(data_len as u32);
            array.copy_from(&data[..]);
            let result = if let Err(err) = self.ws_send.send_with_array_buffer(&array.buffer()) {
                let err = format!("{:?}", err);
                let err = err.split_once("\n").map(|a| a.0.to_string()).unwrap_or(err);
                error!("error sending message: {:?}", err);
                SendResult::Failed(err)
            } else {
                debug!("websocket sent {} bytes", data_len);
                SendResult::Success(data_len)
            };
            ResultInvokable::new(result)
            */
        } else {
            ErrornousInvokable::new(CallError::InvalidTopic)
        }
    }
}
