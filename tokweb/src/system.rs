use js_sys::Promise;
use js_sys::Function;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use wasm_bindgen::JsCast;
use std::future::Future;
use tokterm::api::abi::SystemAbi;
use web_sys::Window;
use web_sys::{console, HtmlElement, HtmlInputElement, Worker};
use web_sys::{Request, RequestInit, RequestMode, Response};
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

use tokterm::err;

pub struct WebSystem
{
}

#[async_trait]
impl SystemAbi
for WebSystem
{
    fn spawn<Fut>(&self, future: Fut)
    where Fut: Future<Output = ()> + Send + 'static
    {
        panic!("not yet implemented");
    }

    fn spawn_blocking<F>(&self, task: F)
    where F: FnOnce() + Send + 'static
    {
        panic!("not yet implemented");
    }

    fn spawn_local<F>(&self, task: F)
    where F: Future<Output = ()> + 'static
    {
        wasm_bindgen_futures::spawn_local(task)
    }

    async fn sleep(&self, ms: i32) {
        let promise = sleep(ms);
        let js_fut = JsFuture::from(promise);
        js_fut.await;
    }

    async fn fetch_file(&self, path: &str) -> Result<Vec<u8>, i32> {
        let url = path.to_string();
        let headers = vec![("Accept".to_string(), "application/wasm".to_string())];
        let (tx, rx) = oneshot::channel();
        self.spawn_local(Box::pin(async move {
            tx.send(system.fetch_file(url.as_str(), "GET", headers, None).await);
        }));
        rx.await.map_err(|_| err::ERR_EIO)?
    }
}

#[wasm_bindgen(module = "/public/worker.js")]
extern "C" {
    #[wasm_bindgen(js_name = "sleep")]
    fn sleep(ms: i32) -> Promise;
}