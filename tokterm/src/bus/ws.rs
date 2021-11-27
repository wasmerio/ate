use async_trait::async_trait;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bus::backend::ws::*;
use web_sys::{MessageEvent, WebSocket as WebSocketSys};
use std::any::type_name;
use wasm_bus::abi::CallError;

use super::*;

pub struct WebSocket {
    rx_msg: broadcast::Receiver<SocketMessage>,
    rx_recv: mpsc::Receiver<Vec<u8>>,
}

impl WebSocket {
    pub fn new(request: Connect) -> Result<(WebSocketInvoker, WebSocketSession), CallError> {
        // Construct the channels
        let (tx_recv, rx_recv) = mpsc::channel(100);
        let (tx_msg, _) = broadcast::channel::<SocketMessage>(100);

        // Open the web socket
        let ws_sys = match WebSocketSys::new(request.url.as_str()) {
            Ok(a) => a,
            Err(err) => {
                debug!("failed to create web socket ({}): {:?}", request.url, err);
                return Err(CallError::Unknown);
            }
        };
        debug!("websocket successfully created");

        let onopen_callback = {
            let tx_msg = tx_msg.clone();
            Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                debug!("websocket open");
                let _ = tx_msg.send(SocketMessage::Opened);
            }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
        };
        ws_sys.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let onclose_callback = {
            let tx_msg = tx_msg.clone();
            Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                debug!("websocket closed");
                let _ = tx_msg.send(SocketMessage::Closed);
            }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
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
                debug!("websocket recv {} bytes (web_sys::Blob)", len);
                if let Err(err) = tx.blocking_send(array.to_vec()) {
                    debug!("websocket bytes silently dropped - {}", err);
                }
            }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
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
                        debug!("websocket bytes silently dropped - {}", err);
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

        let invoker = WebSocketInvoker {
            ws: Some(WebSocket {
                rx_msg: tx_msg.subscribe(),
                rx_recv,
            })
        };
        let session = WebSocketSession {
            ws_send: ws_sys,
        };
        Ok((invoker, session))
    }

    pub async fn run(mut self) {
        loop {
            select! {
                msg = self.rx_msg.recv() => {
                    match msg {
                        Ok(SocketMessage::Opened) => {
                            debug!("websocket successfully opened");
                        }
                        Ok(SocketMessage::Closed) => {
                            debug!("websocket closed before it opened");
                            return;
                        }
                        _ => {
                            debug!("websocket failed before it opened");
                            return;
                        }
                    }
                    break;
                }
                data = self.rx_recv.recv() => {
                    if let Some(data) = data {
                        error!("need to pass the data back to the module here! data = {} bytes", data.len());
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

pub struct WebSocketInvoker
{
    ws: Option<WebSocket>,
}

#[async_trait]
impl Invokable
for WebSocketInvoker
{
    async fn process(&mut self) -> Result<Vec<u8>, CallError>
    {
        let ws = self.ws.take();
        if let Some(ws) = ws {
            let ret = ws.run().await;
            Ok(encrypt_response(&ret)?)
        } else {
            Err(CallError::Unknown)
        }
    }
}

pub struct WebSocketSession
{
    ws_send: WebSocketSys,
}

impl Session
for WebSocketSession
{
    fn call(&mut self, topic: &str, request: &Vec<u8>) -> Box<dyn Invokable + 'static>
    {
        if topic == type_name::<Send>() {
            let request: Send = match decrypt_request(request.as_ref()) {
                Ok(a) => a,
                Err(err) => {
                    return ErrornousInvokable::new(err);
                }
            };
            let data = &request.data;
            let data_len = data.len();
            let array = js_sys::Uint8Array::new_with_length(data_len as u32);
            array.copy_from(&data[..]);
            let result = if let Err(err) = self.ws_send.send_with_array_buffer(&array.buffer()) {
                let err = format!("{:?}", err);
                error!("error sending message: {:?}", err);
                SendResult::Failed(err)
            } else {
                debug!("websocket sent {} bytes", data_len);
                SendResult::Success(data_len)
            };
            ResultInvokable::new(result)
        } else {
            ErrornousInvokable::new(CallError::InvalidTopic)
        }
    }
}