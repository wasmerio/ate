use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bus::backend::ws::*;
use web_sys::{MessageEvent, WebSocket as WebSocketSys};

pub struct WebSocket {
    rx_msg: broadcast::Receiver<SocketMessage>,
    rx_recv: mpsc::Receiver<Vec<u8>>,
}

pub enum WebSocketResult {
    WebSocket(WebSocket),
    Failed,
}

impl WebSocketResult {
    pub async fn run(self) {
        match self {
            WebSocketResult::WebSocket(a) => a.run().await,
            WebSocketResult::Failed => {}
        };
    }
}

impl WebSocket {
    pub fn new(request: Connect) -> WebSocketResult {
        // Construct the channels
        let (_tx_send, mut rx_send) = mpsc::channel(100);
        let (tx_recv, rx_recv) = mpsc::channel(100);
        let (tx_msg, _) = broadcast::channel::<SocketMessage>(100);

        // Open the web socket
        let ws = match WebSocketSys::new(request.url.as_str()) {
            Ok(a) => a,
            Err(err) => {
                debug!("failed to create web socket ({}): {:?}", request.url, err);
                return WebSocketResult::Failed;
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
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let onclose_callback = {
            let tx_msg = tx_msg.clone();
            Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                debug!("websocket closed");
                let _ = tx_msg.send(SocketMessage::Closed);
            }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
        };
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
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
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        let mut rx_msg = tx_msg.subscribe();
        let ws = ws.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match rx_msg.recv().await {
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
            loop {
                tokio::select! {
                    data = rx_send.recv() => {
                        if let Some(data) = data {
                            let data: Vec<u8> = data;
                            let data_len = data.len();
                            let array = js_sys::Uint8Array::new_with_length(data_len as u32);
                            array.copy_from(&data[..]);
                            if let Err(err) = ws.send_with_array_buffer(&array.buffer()) {
                                debug!("error sending message: {:?}", err);
                            } else {
                                debug!("websocket sent {} bytes", data_len);
                            }
                        } else {
                            break;
                        }
                    }
                    msg = rx_msg.recv() => {
                        if Ok(SocketMessage::Closed) == msg {
                            break;
                        }
                    }
                }
            }
            debug!("closing websocket send loop");
        });

        // Return the web socket
        WebSocketResult::WebSocket(WebSocket {
            rx_msg: tx_msg.subscribe(),
            rx_recv,
        })
    }

    pub async fn run(mut self) -> () {
        // Wait for the channel to open (or not)
        loop {
            select! {
                _ = self.rx_msg.recv() => {
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
