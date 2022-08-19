use async_trait::async_trait;
use js_sys::Promise;
use wasmer_os::wasmer::Module;
use wasmer_os::wasmer::Store;
use wasmer_os::wasmer::vm::VMMemory;
use wasmer_os::wasmer_wasi::WasiThreadError;
use std::future::Future;
use std::pin::Pin;
use wasmer_os::api::abi::SystemAbi;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use web_sys::WebGl2RenderingContext;

use super::common::*;
use super::pool::WebThreadPool;
use super::ws::WebSocket;
use wasmer_os::api::*;
use super::webgl::WebGl;
use super::webgl::GlContext;
use super::webgl::WebGlCommand;

pub(crate) enum TerminalCommand {
    Print(String),
    ConsoleRect(mpsc::Sender<ConsoleRect>),
    Cls,
}

pub(crate) struct WebSystem {
    pool: WebThreadPool,
    webgl_tx: mpsc::Sender<WebGlCommand>,
}

impl WebSystem {
    pub(crate) fn new(pool: WebThreadPool, webgl2: WebGl2RenderingContext) -> WebSystem {
        let webgl_tx = GlContext::init(webgl2);

        WebSystem {
            pool,
            webgl_tx,
        }
    }
}

#[async_trait]
impl SystemAbi for WebSystem {
    fn task_shared(
        &self,
        task: Box<
            dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
        >,
    ) {
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let fut = task();
                fut.await;
            })
        }));
    }

    fn task_wasm(
        &self,
        task: Box<dyn FnOnce(Store, Module, Option<VMMemory>) -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
        store: Store,
        module: Module,
        spawn_type: SpawnType,
    ) -> Result<(), WasiThreadError> {
        let run = move |store, module, memory| {
            task(store, module, memory)
        };
        let module_bytes = module.serialize().unwrap();
        self.pool.spawn_wasm(run, store, module_bytes, spawn_type)
    }

    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() + Send + 'static>,
    ) {
        self.pool.spawn_dedicated(task);
    }

    fn task_dedicated_async(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ) {
        self.pool.spawn_dedicated_async(task);
    }

    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>) {
        wasm_bindgen_futures::spawn_local(async move {
            task.await;
        });
    }

    fn sleep(&self, ms: u128) -> AsyncResult<()> {
        let ms = ms as i32;
        let (tx, rx) = mpsc::channel(1);
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let promise = sleep(ms);
                let js_fut = JsFuture::from(promise);

                let _ = js_fut.await;
                let _ = tx.send(()).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Json, rx)
    }

    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, u32>> {
        let url = path.to_string();
        let headers = vec![("Accept".to_string(), "application/wasm".to_string())];

        let (tx, rx) = mpsc::channel(1);
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let ret = crate::common::fetch_data(url.as_str(), "GET", false, None, headers, None).await;
                let _ = tx.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx)
    }

    fn reqwest(
        &self,
        url: &str,
        method: &str,
        options: ReqwestOptions,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, u32>> {
        let url = url.to_string();
        let method = method.to_string();

        let (tx, rx) = mpsc::channel(1);
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let resp = match crate::common::fetch(
                    url.as_str(),
                    method.as_str(),
                    options.gzip,
                    options.cors_proxy,
                    headers,
                    data)
                    .await
                {
                    Ok(a) => a,
                    Err(err) => {
                        let _ = tx.send(Err(err)).await;
                        return;
                    }
                };

                let ok = resp.ok();
                let redirected = resp.redirected();
                let status = resp.status();
                let status_text = resp.status_text();

                let data = match crate::common::get_response_data(resp).await {
                    Ok(a) => a,
                    Err(err) => {
                        let _ = tx.send(Err(err)).await;
                        return;
                    }
                };

                let headers = Vec::new();
                // we can't implement this as the method resp.headers().keys() is missing!
                // how else are we going to parse the headers

                debug!("received {} bytes", data.len());
                let resp = ReqwestResponse {
                    pos: 0,
                    ok,
                    redirected,
                    status,
                    status_text,
                    headers,
                    data: Some(data),
                };
                debug!("response status {}", status);

                let _ = tx.send(Ok(resp)).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx)
    }

    async fn web_socket(&self, url: &str) -> Result<Box<dyn WebSocketAbi>, String> {
        WebSocket::new(url)
    }

    /// Open the WebGL
    async fn webgl(&self) -> Option<Box<dyn WebGlAbi>> {
        Some(Box::new(WebGl::new(&self.webgl_tx)))
    }
}

pub(crate) struct WebConsole {
    term_tx: mpsc::Sender<TerminalCommand>,
}

impl WebConsole {
    pub(crate) fn new(term_tx: mpsc::Sender<TerminalCommand>) -> WebConsole {
        WebConsole { term_tx }
    }
}

#[async_trait]
impl ConsoleAbi for WebConsole {
    async fn stdout(&self, data: Vec<u8>) {
        if let Ok(text) = String::from_utf8(data) {
            let _ = self.term_tx.send(TerminalCommand::Print(text)).await;
        }
    }

    async fn stderr(&self, data: Vec<u8>) {
        if let Ok(text) = String::from_utf8(data) {
            let _ = self.term_tx.send(TerminalCommand::Print(text)).await;
        }
    }

    async fn flush(&self) {}

    async fn log(&self, text: String) {
        console::log(text.as_str());
    }

    async fn console_rect(&self) -> ConsoleRect {
        let (ret_tx, mut ret_rx) = mpsc::channel(1);
        let _ = self
            .term_tx
            .send(TerminalCommand::ConsoleRect(ret_tx))
            .await;
        ret_rx.recv().await.unwrap()
    }

    async fn cls(&self) {
        let _ = self.term_tx.send(TerminalCommand::Cls).await;
    }

    async fn exit(&self) {
        // Web terminals can not exit as they have nowhere to go!
    }
}

#[wasm_bindgen(module = "/js/time.js")]
extern "C" {
    #[wasm_bindgen(js_name = "sleep")]
    fn sleep(ms: i32) -> Promise;
}
