use std::io;
use js_sys::Promise;
use wasmer_wasi::wasmer::Module;
use wasmer_wasi::wasmer::Store;
use wasmer_wasi::wasmer::vm::VMMemory;
use wasmer_wasi::VirtualBus;
use wasmer_wasi::VirtualNetworking;
use wasmer_wasi::WasiCallingId;
use wasmer_wasi::WasiEnv;
use wasmer_wasi::WasiError;
use wasmer_wasi::WasiRuntimeImplementation;
use wasmer_wasi::WasiThreadError;
use wasmer_wasi::WasiTtyState;
use wasmer_wasi::WebSocketAbi;
use wasmer_wasi::runtime::ReqwestOptions;
use wasmer_wasi::runtime::ReqwestResponse;
use wasmer_wasi::runtime::SpawnType;
use wasmer_wasi::types::__WASI_EIO;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

use wasmer_wasi::os::TtyOptions;

use super::common::*;
use super::pool::WebThreadPool;
use super::ws::WebSocket;
#[cfg(feature = "webgl")]
use super::webgl::WebGl;
#[cfg(feature = "webgl")]
use super::webgl::GlContext;
#[cfg(feature = "webgl")]
use super::webgl::WebGlCommand;

pub(crate) enum TerminalCommandRx {
    Print(String),
    Cls,
}

#[derive(Debug)]
pub(crate) struct WebRuntime {
    pool: WebThreadPool,
    term_tx: mpsc::Sender<TerminalCommandRx>,
    #[cfg(feature = "webgl")]
    webgl_tx: mpsc::Sender<WebGlCommand>,
    tty: TtyOptions,
}

impl WebRuntime {
    #[allow(unused_variables)]
    pub(crate) fn new(pool: WebThreadPool, tty_options: TtyOptions, term_tx: mpsc::Sender<TerminalCommandRx>, webgl2: WebGl2RenderingContext) -> WebRuntime {
        #[cfg(feature = "webgl")]
        let webgl_tx = GlContext::init(webgl2);

        WebRuntime {
            pool,
            tty: tty_options,
            term_tx,
            #[cfg(feature = "webgl")]
            webgl_tx,
        }
    }
}

impl VirtualBus<WasiEnv>
for WebRuntime
{

}

impl VirtualNetworking
for WebRuntime
{

}

impl WasiRuntimeImplementation
for WebRuntime {
    fn bus(&self) -> &(dyn VirtualBus<WasiEnv>) {
        self
    }

    fn networking(&self) -> &(dyn VirtualNetworking) {
        self
    }

    fn task_shared(
        &self,
        task: Box<
            dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
        >,
    ) -> Result<(), WasiThreadError> {
        self.pool.spawn_shared(Box::new(move || {
            Box::pin(async move {
                let fut = task();
                fut.await;
            })
        }));
        Ok(())
    }

    fn task_wasm(
        &self,
        task: Box<dyn FnOnce(Store, Module, Option<VMMemory>) + Send + 'static>,
        store: Store,
        module: Module,
        spawn_type: SpawnType,
    ) -> Result<(), WasiThreadError> {
        let module_bytes = module.serialize().unwrap();
        self.pool.spawn_wasm(task, store, module_bytes, spawn_type)?;
        Ok(())
    }

    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), WasiThreadError> {
        self.pool.spawn_dedicated(task);
        Ok(())
    }

    fn task_dedicated_async(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ) -> Result<(), WasiThreadError> {
        self.pool.spawn_dedicated_async(task);
        Ok(())
    }

    /*
    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>) {
        wasm_bindgen_futures::spawn_local(async move {
            task.await;
        });
    }
    */

    fn sleep_now(&self, _id: WasiCallingId, ms: u128) -> Result<(), WasiError> {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
        Ok(())
    }

    /*
    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, u32>> {
        let mut path = path.to_string();
        if path.starts_with("/bin/") == false {
            path = format!("/bin/{}", path);
        }
        if path.ends_with(".wasm") == false {
            path = format!("{}.wasm", path);
        }

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
    */

    fn reqwest(
        &self,
        url: &str,
        method: &str,
        options: ReqwestOptions,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> Result<ReqwestResponse, u32> {
        let url = url.to_string();
        let method = method.to_string();

        let (tx, mut rx) = mpsc::channel(1);
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
        rx.blocking_recv()
            .ok_or(__WASI_EIO as u32)?
    }

    fn web_socket(&self, url: &str) -> Result<Box<dyn WebSocketAbi>, String> {
        WebSocket::new(url)
    }

    /// Open the WebGL
    #[cfg(feature = "webgl")]
    async fn webgl(&self) -> Option<Box<dyn WebGlAbi>> {
        Some(Box::new(WebGl::new(&self.webgl_tx)))
    }

    fn stdout(&self, data: &[u8]) -> io::Result<()> {
        let data = match self.tty.line_feeds() {
            true => {
                data.to_vec()
                    .into_iter()
                    .flat_map(|a| match a {
                        b'\n' => vec![ b'\r', b'\n' ].into_iter(),
                        a => vec![ a ].into_iter()
                    })
                    .collect::<Vec<_>>()
            },
            false => data.to_vec()
        };
        if let Ok(text) = String::from_utf8(data) {
            let _ = self.term_tx.blocking_send(TerminalCommandRx::Print(text));
        }
        Ok(())
    }

    fn stderr(&self, data: &[u8]) -> io::Result<()> {
        self.stdout(data)
    }

    fn log(&self, text: String) -> io::Result<()> {
        console::log(text.as_str());
        Ok(())
    }

    fn tty_get(&self) -> WasiTtyState {
        WasiTtyState {
            cols: self.tty.cols(),
            rows: self.tty.rows(),
            width: 800,
            height: 600,
            stdin_tty: true,
            stdout_tty: true,
            stderr_tty: true,
            echo: self.tty.echo(),
            line_buffered: self.tty.line_buffering(),
            line_feeds: self.tty.line_feeds(),
        }
    }

    fn tty_set(&self, tty_state: WasiTtyState) {
        self.tty.set_cols(tty_state.cols);
        self.tty.set_rows(tty_state.rows);
        self.tty.set_echo(tty_state.echo);
        self.tty.set_line_buffering(tty_state.line_buffered);
        self.tty.set_line_feeds(tty_state.line_feeds);
    }

    fn cls(&self) -> io::Result<()> {
        let _ = self.term_tx.blocking_send(TerminalCommandRx::Cls);
        Ok(())
    }
}

#[wasm_bindgen(module = "/js/time.js")]
extern "C" {
    #[wasm_bindgen(js_name = "sleep")]
    fn sleep(ms: i32) -> Promise;
}
