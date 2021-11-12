#![allow(dead_code)]
#![allow(unused)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;
use wasmer_wasi::{types as wasi_types, WasiFile, WasiFsError};
use wasmer_wasi::vfs::{VirtualFile, FileDescriptor};
use bytes::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::ops::*;

use crate::err;
use crate::common::*;
use crate::fd::*;
use crate::reactor::*;

#[derive(Debug, Clone)]
pub struct TokeraSocketFactory
{
    tx: mpsc::Sender<mpsc::Sender<Fd>>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SocketMessage
{
    Opened,
    Closed
}

impl TokeraSocketFactory
{
    pub fn new(reactor: &Arc<RwLock<Reactor>>) -> TokeraSocketFactory
    {
        let reactor = Arc::clone(reactor);
        let (tx_factory, mut rx_factory) = mpsc::channel::<mpsc::Sender<Fd>>(10);
        wasm_bindgen_futures::spawn_local(async move {
            while let Some(tx_request) = rx_factory.recv().await {
                let reactor = reactor.clone();
                let (mut fd, tx, mut rx) = {
                    let mut reactor = reactor.write().await;
                    match reactor.bidirectional(MAX_MPSC, MAX_MPSC, ReceiverMode::Message(false)) {
                        Ok((fd, tx, rx)) => {
                            (
                                Fd::new(fd, reactor.deref()),
                                tx,
                                rx
                            )
                        },
                        Err(err) => {
                            debug!("failed to create file handle for web connection: {:?}", err);
                            continue;    
                        }
                    }
                };
                fd.set_blocking(false);

                // Give the open channel back to the caller
                tx_request.send(fd.clone()).await;

                // Now we wait for the connection type and spawn based of it
                wasm_bindgen_futures::spawn_local(async move {
                    use wasi_net::web_command::WebCommand;

                    let req = if let Some(a) = read_line(&mut rx).await {
                        a
                    } else {
                        debug!("failed to read command string from /dev/web");
                        return;
                    };
                    match WebCommand::deserialize(req.as_str()) {
                        Ok(WebCommand::WebSocket { url }) => {
                            open_web_socket(fd, url.as_str(), reactor, rx, tx).await;
                        },
                        Ok(WebCommand::WebRequest {
                            url,
                            method,
                            headers,
                            body
                        }) => {
                            open_web_request(fd, url.as_str(), method.as_str(), headers, body, reactor, rx, tx).await;
                        },
                        Err(err) => {
                            debug!("failed to deserialize the command");
                            return;
                        }
                    };
                });
            }
        });

        TokeraSocketFactory {
            tx: tx_factory
        }
    }

    pub fn create(&self) -> Fd {
        let (tx, mut rx) = mpsc::channel(1);
        self.tx.blocking_send(tx);
        rx.blocking_recv().unwrap()
    }
}

async fn read_line(rx: &mut mpsc::Receiver<Vec<u8>>) -> Option<String>
{
    let mut line = String::new();
    loop {
        if let Some(a) = rx.recv().await {
            match String::from_utf8(a) {
                Ok(a) => {
                    line += a.as_str();
                    if line.ends_with("\n") {
                        break;
                    }
                },
                Err(_err) => {
                    return None;
                }
            };
        } else {
            return None;
        }
    }
    Some(line.trim().to_string())
}

async fn open_web_socket(fd: Fd, url: &str, reactor: Arc<RwLock<Reactor>>, mut rx: mpsc::Receiver<Vec<u8>>, tx: mpsc::Sender<Vec<u8>>) {
    fd.set_blocking(false);
    let ws = match WebSocket::new(url) {
        Ok(a) => a,
        Err(err) => {
            debug!("failed to create web socket ({}): {:?}", url, err);
            return;
        }
    };
    debug!("websocket successfully created");
    
    let (tx_msg, mut rx_msg) = broadcast::channel::<SocketMessage>(100);
    
    let onopen_callback = {
        let tx_msg = tx_msg.clone();
        Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            debug!("websocket open");
            tx_msg.send(SocketMessage::Opened);
            crate::wasi::inc_idle_ver();
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
    };
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    let onclose_callback = {
        let reactor = reactor.clone();
        let tx_msg = tx_msg.clone();
        let fd = fd.raw;
        Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            debug!("websocket closed");
            {
                let reactor = reactor.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut reactor = reactor.write().await;
                    reactor.remove_pipe(fd);
                });
            }
            tx_msg.send(SocketMessage::Closed);
            crate::wasi::inc_idle_ver();
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
    };
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    {
        let mut rx_msg = tx_msg.subscribe();
        
        let ws = ws.clone();                    
        let fd = fd.raw;
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
                tokio::select!
                {
                    data = rx.recv() => {
                        if let Some(data) = data {
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
    }

    let fr = web_sys::FileReader::new().unwrap();
    let fr_c = fr.clone();
    let onloadend_cb = {
        let tx = tx.clone();
        Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
            let len = array.byte_length() as usize;
            debug!("websocket recv {} bytes", len);
            tx.blocking_send(array.to_vec());
            crate::wasi::inc_idle_ver();
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
    };
    fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));                            
    onloadend_cb.forget();

    let onmessage_callback = {
        let tx = tx.clone();
        Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let data = js_sys::Uint8Array::new(&abuf).to_vec();
                debug!("websocket recv {} bytes", data.len());
                tx.blocking_send(data);
                crate::wasi::inc_idle_ver();
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

    // Wait for the channel to open (or not)
    let _ = rx_msg.recv().await;
}

async fn open_web_request(fd: Fd, url: &str, method: &str, headers: Vec<(String, String)>, data: Option<Vec<u8>>, reactor: Arc<RwLock<Reactor>>, mut rx: mpsc::Receiver<Vec<u8>>, tx: mpsc::Sender<Vec<u8>>) -> Result<(), i32> {
    debug!("executing HTTP {}", method);

    let ret = fetch(url, method, headers, data).await?;
    debug!("received {} bytes", ret.len());

    let _ = tx.send(ret).await;
    Ok(())
}