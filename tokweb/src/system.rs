#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use js_sys::Promise;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use term_lib::api::abi::SystemAbi;
use term_lib::err;
use tokio::sync::oneshot;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use std::rc::Rc;
use std::cell::RefCell;

use super::pool::WebThreadPool;
use super::ws::WebSocket;
use term_lib::api::*;

pub struct WebSystem {
    pool: WebThreadPool,
}

impl WebSystem {
    pub fn new(pool: WebThreadPool) -> WebSystem {
        WebSystem { pool }
    }
}

impl SystemAbi for WebSystem {
    fn task_shared(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ) {
        self.pool.spawn_shared(task);
    }

    fn task_stateful(
        &self,
        task: Box<dyn FnOnce(Rc<RefCell<ThreadLocal>>) -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
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

    fn sleep(&self, ms: i32) -> Pin<Box<dyn Future<Output = ()>>> {
        let promise = sleep(ms);
        let js_fut = JsFuture::from(promise);
        Box::pin(async move {
            let _ = js_fut.await;
        })
    }

    fn fetch_file(&self, path: &str) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, i32>>>> {
        let url = path.to_string();
        let headers = vec![("Accept".to_string(), "application/wasm".to_string())];
        let (tx, rx) = oneshot::channel();
        self.task_local(Box::pin(async move {
            let _ = tx.send(crate::common::fetch_data(url.as_str(), "GET", headers, None).await);
        }));
        Box::pin(async move { rx.await.map_err(|_| err::ERR_EIO)? })
    }

    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<ReqwestResponse, i32>>>> {
        let url = url.to_string();
        let method = method.to_string();
        Box::pin(async move {
            let resp = crate::common::fetch(url.as_str(), method.as_str(), headers, data).await?;

            let resp = ReqwestResponse {
                ok: resp.ok(),
                redirected: resp.redirected(),
                status: resp.status(),
                status_text: resp.status_text(),
                data: crate::common::get_response_data(resp).await?,
            };
            Ok(resp)
        })
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
