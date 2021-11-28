use crate::common::MAX_MPSC;
use async_trait::async_trait;
use tokio::sync::mpsc;
use wasm_bus::abi::CallError;
use wasm_bus::backend::time::Sleep;

use super::*;
use crate::api::System;

struct TimeDelay {
    duration_ms: u128,
    result: mpsc::Sender<()>,
}

#[derive(Debug, Clone)]
pub struct TimeFactory {
    maker: mpsc::Sender<TimeDelay>,
    system: System,
}

impl TimeFactory {
    pub fn new() -> TimeFactory {
        let system = System::default();
        let (tx_factory, mut rx_factory) = mpsc::channel::<TimeDelay>(MAX_MPSC);
        system.spawn_local(async move {
            while let Some(create) = rx_factory.recv().await {
                system.spawn_local(async move {
                    let _ = system.sleep(create.duration_ms as i32).await;
                    let _ = create.result.send(()).await;
                });
            }
        });
        TimeFactory { system, maker: tx_factory }
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
