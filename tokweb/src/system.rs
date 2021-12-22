use async_trait::async_trait;
use js_sys::Promise;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use term_lib::api::abi::SystemAbi;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

use super::common::*;
use super::pool::WebThreadPool;
use super::ws::WebSocket;
use term_lib::api::*;
use wasm_bus::backend::reqwest::Response as ReqwestResponse;

pub(crate) enum TerminalCommand {
    Print(String),
    ConsoleRect(mpsc::Sender<ConsoleRect>),
    Cls,
}

pub(crate) struct WebSystem {
    pool: WebThreadPool,
    term_tx: mpsc::Sender<TerminalCommand>,
}

impl WebSystem {
    pub(crate) fn new(pool: WebThreadPool, term_tx: mpsc::Sender<TerminalCommand>) -> WebSystem {
        WebSystem { pool, term_tx }
    }
}

#[async_trait]
impl SystemAbi for WebSystem {
    fn task_shared(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static>,
    ) {
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let fut = task();
                fut.await;
            })
        }));
    }

    fn task_stateful(
        &self,
        task: Box<
            dyn FnOnce(Rc<RefCell<ThreadLocal>>) -> Pin<Box<dyn Future<Output = ()> + 'static>>
                + Send
                + 'static,
        >,
    ) {
        self.pool.spawn_stateful(task);
    }

    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ) {
        self.pool.spawn_dedicated(task);
    }

    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>) {
        wasm_bindgen_futures::spawn_local(async move {
            task.await;
        });
    }

    fn sleep(&self, ms: i32) -> AsyncResult<()> {
        let (tx, rx) = mpsc::channel(1);
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let promise = sleep(ms);
                let js_fut = JsFuture::from(promise);
    
                let _ = js_fut.await;
                let _ = tx.send(()).await;
            })
        }));
        AsyncResult::new(rx)
    }

    async fn print(&self, text: String) {
        let _ = self.term_tx.send(TerminalCommand::Print(text)).await;
    }

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

    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, i32>> {
        let url = path.to_string();
        let headers = vec![("Accept".to_string(), "application/wasm".to_string())];

        let (tx, rx) = mpsc::channel(1);
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let ret = crate::common::fetch_data(url.as_str(), "GET", headers, None).await;
                let _ = tx.send(ret).await;
            })
        }));
        AsyncResult::new(rx)
    }

    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, i32>> {
        let url = url.to_string();
        let method = method.to_string();

        let (tx, rx) = mpsc::channel(1);
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let resp = match crate::common::fetch(url.as_str(), method.as_str(), headers, data)
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
        AsyncResult::new(rx)
    }

    fn web_socket(&self, url: &str) -> Result<Arc<dyn WebSocketAbi>, String> {
        WebSocket::new(url)
    }
}

#[wasm_bindgen(module = "/public/worker.js")]
extern "C" {
    #[wasm_bindgen(js_name = "sleep")]
    fn sleep(ms: i32) -> Promise;
}
