use async_trait::async_trait;
use include_dir::{include_dir, Dir};
use std::cell::RefCell;
use std::convert::TryFrom;
use std::future::Future;
use std::io::{self, Write};
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use term_lib::api::abi::*;
use term_lib::api::AsyncResult;
use term_lib::api::ThreadLocal;
use term_lib::api::WebSocketAbi;
use term_lib::err;
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::reqwest::Response as ReqwestResponse;

static PUBLIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/public");

thread_local!(static THREAD_LOCAL: Rc<RefCell<ThreadLocal>> = Rc::new(RefCell::new(ThreadLocal::default())));

#[derive(Debug, Clone)]
pub struct SysSystem {
    runtime: Arc<Runtime>,
}

impl SysSystem {
    pub fn new() -> SysSystem {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        SysSystem {
            runtime: Arc::new(runtime),
        }
    }

    pub fn run(&self) {
        let (_wait_forever_tx, mut wait_forever_rx) = mpsc::channel::<()>(1);
        self.runtime.block_on(async move {
            let _ = wait_forever_rx.recv().await;
        });
    }
}

#[async_trait]
impl SystemAbi for SysSystem {
    /// Starts an asynchronous task that will run on a shared worker pool
    /// This task must not block the execution or it could cause a deadlock
    fn task_shared(
        &self,
        task: Box<
            dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
        >,
    ) {
        self.runtime.spawn(async move {
            let fut = task();
            fut.await
        });
    }

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool that has a stateful thread local variable
    /// It is ok for this task to block execution and any async futures within its scope
    fn task_stateful(
        &self,
        task: Box<
            dyn FnOnce(Rc<RefCell<ThreadLocal>>) -> Pin<Box<dyn Future<Output = ()> + 'static>>
                + Send
                + 'static,
        >,
    ) {
        let rt = self.runtime.clone();
        self.runtime.spawn_blocking(move || {
            THREAD_LOCAL.with(|local| {
                let fut = task(local.clone());
                let set = tokio::task::LocalSet::new();
                set.block_on(rt.deref(), fut);
                rt.block_on(set);
            });
        });
    }

    /// Starts an asynchronous task will will run on a dedicated thread
    /// pulled from the worker pool. It is ok for this task to block execution
    /// and any async futures within its scope
    fn task_dedicated(
        &self,
        task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + 'static>> + Send + 'static>,
    ) {
        let rt = self.runtime.clone();
        self.runtime.spawn_blocking(move || {
            let fut = task();
            let set = tokio::task::LocalSet::new();
            set.block_on(rt.deref(), fut);
            rt.block_on(set);
        });
    }

    /// Starts an asynchronous task on the current thread. This is useful for
    /// launching background work with variables that are not Send.
    fn task_local(&self, task: Pin<Box<dyn Future<Output = ()> + 'static>>) {
        tokio::task::spawn_local(async move {
            task.await;
        });
    }

    /// Puts the current thread to sleep for a fixed number of milliseconds
    fn sleep(&self, ms: i32) -> AsyncResult<()> {
        let (tx_done, rx_done) = mpsc::channel(1);
        self.task_shared(Box::new(move || {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(ms as u64)).await;
                let _ = tx_done.send(()).await;
            })
        }));
        AsyncResult::new(rx_done)
    }

    async fn print(&self, text: String) {
        let _ = io::stdout().lock().write_all(text.as_bytes());
    }

    /// Writes output to the log
    async fn log(&self, text: String) {
        let _ = io::stderr().lock().write_all(text.as_bytes());
    }

    /// Gets the number of columns and rows in the terminal
    async fn console_rect(&self) -> ConsoleRect {
        if let Some((w, h)) = term_size::dimensions() {
            ConsoleRect {
                cols: w as u32,
                rows: h as u32,
            }
        } else {
            ConsoleRect { cols: 80, rows: 25 }
        }
    }

    /// Clears the terminal
    async fn cls(&self) {
        print!("{}[2J", 27 as char);
    }

    /// Fetches a data file from the local context of the process
    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, i32>> {
        let path = path.to_string();
        let (tx_done, rx_done) = mpsc::channel(1);
        self.task_dedicated(Box::new(move || {
            Box::pin(async move {
                let ret = PUBLIC_DIR
                    .get_file(path.as_str())
                    .map_or(Err(err::ERR_ENOENT), |file| Ok(file.contents().to_vec()));
                let _ = tx_done.send(ret).await;
            })
        }));
        AsyncResult::new(rx_done)
    }

    /// Performs a HTTP or HTTPS request to a destination URL
    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, i32>> {
        let method = method.to_string();
        let url = url.to_string();

        let (tx_done, rx_done) = mpsc::channel(1);
        self.task_shared(Box::new(move || {
            Box::pin(async move {
                let ret = move || async move {
                    let method = reqwest::Method::try_from(method.as_str()).map_err(|err| {
                        debug!("failed to convert method ({}) - {}", method, err);
                        err::ERR_EIO
                    })?;

                    let client = reqwest::ClientBuilder::default().build().map_err(|err| {
                        debug!("failed to build reqwest client - {}", err);
                        err::ERR_EIO
                    })?;

                    let mut builder = client.request(method, url.as_str());
                    for (header, val) in headers {
                        if let Ok(header) =
                            reqwest::header::HeaderName::from_bytes(header.as_bytes())
                        {
                            builder = builder.header(header, val);
                        } else {
                            debug!("failed to parse header - {}", header);
                        }
                    }

                    if let Some(data) = data {
                        builder = builder.body(reqwest::Body::from(data));
                    }

                    let request = builder.build().map_err(|err| {
                        debug!("failed to convert request (url={}) - {}", url.as_str(), err);
                        err::ERR_EIO
                    })?;

                    let response = client.execute(request).await.map_err(|err| {
                        debug!("failed to execute reqest - {}", err);
                        err::ERR_EIO
                    })?;

                    let status = response.status().as_u16();
                    let status_text = response.status().as_str().to_string();
                    let data = response.bytes().await.map_err(|err| {
                        debug!("failed to read response bytes - {}", err);
                        err::ERR_EIO
                    })?;
                    let data = data.to_vec();

                    Ok(ReqwestResponse {
                        pos: 0usize,
                        ok: true,
                        status,
                        status_text,
                        redirected: false,
                        data: Some(data),
                        headers: Vec::new(),
                    })
                };
                let ret = ret().await;
                let _ = tx_done.send(ret).await;
            })
        }));
        AsyncResult::new(rx_done)
    }

    fn web_socket(&self, _url: &str) -> Result<Arc<dyn WebSocketAbi>, String> {
        return Err("not implemented".to_string());
    }
}
