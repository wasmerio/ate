#![allow(dead_code)]
#![allow(unused)]
use bytes::*;
use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::ops::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasi_net::backend::StdioMode;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bus::backend::ws::SocketMessage;
use wasmer_vfs::{FileDescriptor, VirtualFile};
use wasmer_wasi::{types as wasi_types, WasiFile, WasiFsError};
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

use crate::bin::*;
use crate::common::*;
use crate::console::*;
use crate::err;
use crate::eval::*;
use crate::fd::*;
use crate::pipe::*;
use crate::pipe::*;
use crate::pool::*;
use crate::reactor::*;
use crate::state::*;
use crate::stdio::*;
use crate::tty::*;

#[derive(Debug, Clone)]
pub struct TokeraSocket {
    tx: mpsc::Sender<mpsc::Sender<Fd>>,
}

impl TokeraSocket {
    pub fn new(
        reactor: &Arc<RwLock<Reactor>>,
        exec_factory: ExecFactory,
        inherit_stdin: Fd,
        inherit_stdout: Fd,
        inherit_stderr: Fd,
    ) -> TokeraSocket {
        let reactor = Arc::clone(reactor);
        let (tx_factory, mut rx_factory) = mpsc::channel::<mpsc::Sender<Fd>>(10);
        wasm_bindgen_futures::spawn_local(async move {
            while let Some(tx_request) = rx_factory.recv().await {
                let (mut fd, tx, mut rx) =
                    bidirectional(MAX_MPSC, MAX_MPSC, ReceiverMode::Message(false));
                fd.set_blocking(false);

                // Give the open channel back to the caller
                tx_request.send(fd.clone()).await;

                // Now we wait for the connection type and spawn based of it
                let reactor = Arc::clone(&reactor);
                let exec_factory = exec_factory.clone();
                let inherit_stdin = inherit_stdin.clone();
                let inherit_stdout = inherit_stdout.clone();
                let inherit_stderr = inherit_stderr.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    use wasi_net::backend::*;

                    let req = if let Some(a) = read_line(&mut rx).await {
                        a
                    } else {
                        debug!("failed to read command string from /dev/web");
                        return;
                    };
                    match Command::deserialize(req.as_str()) {
                        Ok(Command::WebSocketVersion1 { url }) => {
                            open_web_socket(fd, url.as_str(), reactor, rx, tx).await;
                        }
                        Ok(Command::WebRequestVersion1 {
                            url,
                            method,
                            headers,
                            body,
                        }) => {
                            open_web_request(
                                fd,
                                url.as_str(),
                                method.as_str(),
                                headers,
                                body,
                                reactor,
                                rx,
                                tx,
                            )
                            .await;
                        }
                        Ok(Command::SpawnProcessVersion1 {
                            path,
                            args,
                            current_dir,
                            stdin_mode,
                            stdout_mode,
                            stderr_mode,
                            pre_open,
                        }) => {
                            open_exec_request(
                                fd,
                                path,
                                args,
                                current_dir,
                                pre_open,
                                exec_factory.clone(),
                                stdin_mode,
                                stdout_mode,
                                stderr_mode,
                                &inherit_stdin,
                                &inherit_stdout,
                                &inherit_stderr,
                                reactor,
                                rx,
                                tx,
                            )
                            .await;
                        }
                        Err(err) => {
                            debug!("failed to deserialize the command");
                            return;
                        }
                    };
                });
            }
        });

        TokeraSocket { tx: tx_factory }
    }

    pub fn create(&self) -> Fd {
        let (tx, mut rx) = mpsc::channel(1);
        self.tx.blocking_send(tx);
        rx.blocking_recv().unwrap()
    }
}

async fn read_line(rx: &mut mpsc::Receiver<Vec<u8>>) -> Option<String> {
    let mut line = String::new();
    loop {
        if let Some(a) = rx.recv().await {
            match String::from_utf8(a) {
                Ok(a) => {
                    line += a.as_str();
                    if line.ends_with("\n") {
                        break;
                    }
                }
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

async fn open_web_socket(
    fd: Fd,
    url: &str,
    reactor: Arc<RwLock<Reactor>>,
    mut rx: mpsc::Receiver<Vec<u8>>,
    tx: mpsc::Sender<Vec<u8>>,
) {
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
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
    };
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    let onclose_callback = {
        let reactor = reactor.clone();
        let tx_msg = tx_msg.clone();
        Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            debug!("websocket closed");
            tx_msg.send(SocketMessage::Closed);
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
    };
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    {
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
            debug!("websocket recv {} bytes (web_sys::Blob)", len);
            if let Err(err) = tx.blocking_send(array.to_vec()) {
                debug!("websocket bytes silently dropped - {}", err);
            }
        }) as Box<dyn FnMut(web_sys::ProgressEvent)>)
    };
    fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
    onloadend_cb.forget();

    // Before we attach the message process let the caller know its all
    // running along nicely
    let ret = wasi_net::backend::Response::WebSocketVersion1 {};
    let mut ret = match ret.serialize() {
        Ok(a) => a,
        Err(err) => {
            debug!("websocket failed serialize the web response");
            return;
        }
    };
    ret += "\n";
    tx.blocking_send(ret.into_bytes());

    // Attach the message process
    let onmessage_callback = {
        let tx = tx.clone();
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

    // Wait for the channel to open (or not)
    let _ = rx_msg.recv().await;
}

async fn open_web_request(
    fd: Fd,
    url: &str,
    method: &str,
    headers: Vec<(String, String)>,
    data: Option<Vec<u8>>,
    reactor: Arc<RwLock<Reactor>>,
    mut rx: mpsc::Receiver<Vec<u8>>,
    tx: mpsc::Sender<Vec<u8>>,
) -> Result<(), i32> {
    debug!("executing HTTP {}", method);

    let resp = fetch(url, method, headers, data).await?;
    debug!("response status {}", resp.status());

    let headers = Vec::new();
    // we can't implement this as the method resp.headers().keys() is missing!
    // how else are we going to parse the headers

    let ret = wasi_net::backend::Response::WebRequestVersion1 {
        ok: resp.ok(),
        redirected: resp.redirected(),
        status: resp.status(),
        status_text: resp.status_text(),
        headers,
        has_data: true,
    };
    let mut ret = match ret.serialize() {
        Ok(a) => a,
        Err(err) => {
            debug!("websocket failed serialize the web response");
            return Ok(());
        }
    };
    ret += "\n";
    let _ = tx.send(ret.into_bytes()).await;

    let ret = get_response_data(resp).await?;
    debug!("received {} bytes", ret.len());

    let _ = tx.send(ret).await;
    let _ = rx.recv().await;

    Ok(())
}

async fn open_exec_request(
    mut fd: Fd,
    path: String,
    args: Vec<String>,
    current_dir: Option<String>,
    pre_open: Vec<String>,
    factory: ExecFactory,
    stdin_mode: StdioMode,
    stdout_mode: StdioMode,
    stderr_mode: StdioMode,
    inherit_stdin: &Fd,
    inherit_stdout: &Fd,
    inherit_stderr: &Fd,
    reactor: Arc<RwLock<Reactor>>,
    mut rx: mpsc::Receiver<Vec<u8>>,
    tx: mpsc::Sender<Vec<u8>>,
) -> Result<(), i32> {
    use wasi_net::backend::MessageProcess;
    debug!("executing process {}", path);

    // Switch back to blocking mode
    fd.set_blocking(true);

    // Build the comand string
    let mut cmd = path.clone();
    for arg in args {
        cmd.push_str(" ");
        if arg.contains(" ") && cmd.starts_with("\"") == false && cmd.starts_with("'") == false {
            cmd.push_str("\"");
            cmd.push_str(&arg);
            cmd.push_str("\"");
        } else {
            cmd.push_str(&arg);
        }
    }

    // Get the current job (if there is none then fail)
    let job = {
        let reactor = reactor.read().await;
        reactor.get_current_job().ok_or(err::ERR_ECHILD)?
    };

    // Create all the stdio
    let (stdin, stdin_tx) = pipe_in(ReceiverMode::Stream);
    let (stdout, stdout_rx) = pipe_out();
    let (stderr, stderr_rx) = pipe_out();

    // Perform hooks back to the main stdio
    let (stdin, mut stdin_tx) = match stdin_mode {
        StdioMode::Null => (stdin, None),
        StdioMode::Inherit => (inherit_stdin.clone(), None),
        StdioMode::Piped => (stdin, Some(stdin_tx)),
    };
    let (stdout, mut stdout_rx) = match stdout_mode {
        StdioMode::Null => (stdout, None),
        StdioMode::Inherit => (inherit_stdout.clone(), None),
        StdioMode::Piped => (stdout, Some(stdout_rx)),
    };
    let (stderr, mut stderr_rx) = match stderr_mode {
        StdioMode::Null => (stderr, None),
        StdioMode::Inherit => (inherit_stderr.clone(), None),
        StdioMode::Piped => (stderr, Some(stderr_rx)),
    };

    // Build a context
    let ctx = SpawnContext::new(
        cmd,
        job.env.deref().clone(),
        job.clone(),
        stdin,
        stdout,
        stderr,
        current_dir.unwrap_or(job.working_dir.clone()),
        pre_open,
        job.root.clone(),
    );

    // Start the process
    let mut rx = factory.spawn(ctx).await;

    // Declare a function that will send the message over the socket
    async fn send(fd: &mut Fd, msg: MessageProcess) {
        if let Ok(mut submit) = msg.serialize() {
            submit += "\n";
            fd.write_vec(submit.into_bytes()).await;
        }
    }

    // Now process all the STDIO concurrently
    if let Some(mut stdin_tx) = stdin_tx.as_mut() {
        if let Some(mut stdout_rx) = stdout_rx.as_mut() {
            if let Some(mut stderr_rx) = stderr_rx.as_mut() {
                loop {
                    tokio::select! {
                        data = stdout_rx.recv() => {
                            if let Some(data) = data {
                                send(&mut fd, MessageProcess::Stdout(data)).await;
                            } else {
                                break;
                            }
                        }
                        data = stderr_rx.recv() => {
                            if let Some(data) = data {
                                send(&mut fd, MessageProcess::Stderr(data)).await;
                            } else {
                                break;
                            }
                        }
                        data = fd.read_async() => {
                            if let Ok(data) = data {
                                stdin_tx.send(data).await;
                            } else {
                                break;
                            }
                        }
                    }
                }
            } else {
                loop {
                    tokio::select! {
                        data = stdout_rx.recv() => {
                            if let Some(data) = data {
                                send(&mut fd, MessageProcess::Stdout(data)).await;
                            } else {
                                break;
                            }
                        }
                        data = fd.read_async() => {
                            if let Ok(data) = data {
                                stdin_tx.send(data).await;
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // If we exit the concurrent loop we need to sequentially send
    // the data as well
    if let Some(mut stdin_tx) = stdin_tx.as_mut() {
        while let Ok(data) = fd.read_async().await {
            stdin_tx.send(data).await;
        }
    }
    if let Some(mut stdout_rx) = stdout_rx.as_mut() {
        while let Some(data) = stdout_rx.recv().await {
            send(&mut fd, MessageProcess::Stdout(data)).await;
        }
    }
    if let Some(mut stderr_rx) = stderr_rx.as_mut() {
        while let Some(data) = stderr_rx.recv().await {
            send(&mut fd, MessageProcess::Stderr(data)).await;
        }
    }

    // Wait for the process to exit then send that
    send(
        &mut fd,
        match rx.await {
            Ok(EvalPlan::Executed { code, .. }) => MessageProcess::Exited(code),
            Ok(EvalPlan::InternalError) => MessageProcess::Exited(err::ERR_ENOEXEC),
            Ok(EvalPlan::Invalid) => MessageProcess::Exited(err::ERR_EINVAL),
            Ok(EvalPlan::MoreInput) => MessageProcess::Exited(err::ERR_EINVAL),
            Err(err) => MessageProcess::Exited(err::ERR_EPIPE),
        },
    )
    .await;

    // We are done (this will close all the pipes)
    Ok(())
}
