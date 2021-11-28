use crate::common::MAX_MPSC;
use async_trait::async_trait;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::backend::reqwest::*;

use crate::api::System;
use super::*;
use crate::common::*;

struct WebRequestCreate {
    request: Request,
    result: mpsc::Sender<Result<Response, i32>>,
}

#[derive(Debug, Clone)]
pub struct WebRequestFactory {
    maker: mpsc::Sender<WebRequestCreate>,
}

impl WebRequestFactory {
    pub fn new() -> WebRequestFactory {
        let system = System::default();
        let (tx_factory, mut rx_factory) = mpsc::channel::<WebRequestCreate>(MAX_MPSC);
        system.spawn_local(async move {
            while let Some(create) = rx_factory.recv().await {
                system.spawn_local(async move {
                    let url = create.request.url;
                    let method = create.request.method;
                    let headers = create.request.headers;
                    let data = create.request.body;

                    let resp = move || async move {
                        debug!("executing HTTP {}", method);

                        let resp = fetch(&url, &method, headers, data).await?;

                        let headers = Vec::new();
                        // we can't implement this as the method resp.headers().keys() is missing!
                        // how else are we going to parse the headers

                        // Grab all the data from the response
                        let ok = resp.ok();
                        let redirected = resp.redirected();
                        let status = resp.status();
                        let status_text = resp.status_text();
                        let data = get_response_data(resp).await?;
                        debug!("received {} bytes", data.len());

                        let resp = Response {
                            ok,
                            redirected,
                            status,
                            status_text,
                            headers,
                            data: Some(data),
                        };
                        debug!("response status {}", status);
                        Ok(resp)
                    };
                    let _ = create.result.send(resp().await).await;
                });
            }
        });
        WebRequestFactory { maker: tx_factory }
    }

    pub fn create(&self, request: Request) -> WebRequestInvokable {
        let (tx_result, rx_result) = mpsc::channel(1);
        let request = WebRequestCreate {
            request,
            result: tx_result,
        };
        let _ = self.maker.blocking_send(request);
        WebRequestInvokable { rx: rx_result }
    }
}

pub struct WebRequestInvokable {
    rx: mpsc::Receiver<Result<Response, i32>>,
}

#[async_trait]
impl Invokable for WebRequestInvokable {
    async fn process(&mut self) -> Result<Vec<u8>, CallError> {
        if let Some(ret) = self.rx.recv().await {
            Ok(encode_response(&ret)?)
        } else {
            Err(CallError::Aborted)
        }
    }
}
