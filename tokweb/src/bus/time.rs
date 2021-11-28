use crate::common::MAX_MPSC;
use async_trait::async_trait;
use js_sys::Promise;
use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use wasm_bus::abi::CallError;
use wasm_bus::backend::time::Sleep;

use super::*;

struct TimeDelay {
    duration_ms: u128,
    result: mpsc::Sender<()>,
}

#[derive(Debug, Clone)]
pub struct TimeFactory {
    maker: mpsc::Sender<TimeDelay>,
}

impl TimeFactory {
    pub fn new() -> TimeFactory {
        let (tx_factory, mut rx_factory) = mpsc::channel::<TimeDelay>(MAX_MPSC);
        wasm_bindgen_futures::spawn_local(async move {
            while let Some(create) = rx_factory.recv().await {
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = timer(create.duration_ms as i32).await;
                    let _ = create.result.send(()).await;
                });
            }
        });
        TimeFactory { maker: tx_factory }
    }

    pub fn create(&self, request: Sleep) -> SleepInvokable {
        let (tx_result, rx_result) = mpsc::channel(1);
        let request = TimeDelay {
            duration_ms: request.duration_ms,
            result: tx_result,
        };
        let _ = self.maker.blocking_send(request);
        SleepInvokable { rx: rx_result }
    }
}

pub struct SleepInvokable {
    rx: mpsc::Receiver<()>,
}

#[async_trait]
impl Invokable for SleepInvokable {
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        let _ = self.rx.recv().await;
        let ret = ();
        Ok(encode_response(&ret)?)
    }
}

async fn timer(ms: i32) -> Result<(), JsValue> {
    let promise = sleep(ms);
    let js_fut = JsFuture::from(promise);
    let _ = js_fut.await?;
    Ok(())
}

#[wasm_bindgen(module = "/public/worker.js")]
extern "C" {
    #[wasm_bindgen(js_name = "sleep")]
    fn sleep(ms: i32) -> Promise;
}
