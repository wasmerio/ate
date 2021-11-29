#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket as WebSocketSys};
use tokterm::api::*;
use std::sync::Arc;
use std::ops::*;

#[derive(Clone)]

pub struct WebSocket
{
    sys: WebSocketSys
}

impl WebSocket
{
    pub fn new(url: &str) -> Result<Arc<dyn WebSocketAbi>, String>
    {
        // Open the web socket
        let ws_sys = WebSocketSys::new(url)
            .map_err(|err| format!("{:?}", err))?;

        Ok(Arc::new(WebSocket {
            sys: ws_sys
        }))
    }
}

impl WebSocketAbi
for WebSocket
{
    fn set_onopen(&self, mut callback: Box<dyn FnMut()>)
    {
        let callback = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            callback.deref_mut()();
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>);
        self.sys.set_onopen(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
    }

    fn set_onclose(&self, callback: Box<dyn Fn()>)
    {
        let callback = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            callback.deref()();
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>);
        self.sys.set_onclose(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
    }

    fn set_onmessage(&self, callback: Box<dyn Fn(Vec<u8>)>)
    {
        let callback = Arc::new(callback);

        let fr = web_sys::FileReader::new().unwrap();
        let fr_c = fr.clone();
        let onloadend_cb = {
            let callback = callback.clone();
            Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                let data = array.to_vec();
                callback.deref()(data);
            }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
        };
        fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
        onloadend_cb.forget();

        // Attach the message process
        let onmessage_callback = {
            let callback = callback.clone();
            Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let data = js_sys::Uint8Array::new(&abuf).to_vec();
                    callback.deref()(data);
                } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                    fr.read_as_array_buffer(&blob).expect("blob not readable");
                } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    debug!("message event, received Text: {:?}", txt);
                } else {
                    debug!("websocket received unknown message type");
                }
            }) as Box<dyn FnMut(MessageEvent)>)
        };
        self.sys.set_binary_type(web_sys::BinaryType::Arraybuffer);
        self.sys.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
    }

    fn send(&self, data: Vec<u8>) -> Result<(), String>
    {
        let data_len = data.len();
        let array =
            js_sys::Uint8Array::new_with_length(data_len as u32);
        array.copy_from(&data[..]);
        self.sys.send_with_array_buffer(&array.buffer())
            .map_err(|err| format!("{:?}", err))
    }
}