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
use term_lib::api::SerializationFormat;
use term_lib::api::ThreadLocal;
use term_lib::api::WebSocketAbi;
use term_lib::err;
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::ws::SysWebSocket;

#[cfg(feature="embedded_files")]
static PUBLIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/public");

thread_local!(static THREAD_LOCAL: Rc<RefCell<ThreadLocal>> = Rc::new(RefCell::new(ThreadLocal::default())));

#[derive(Debug, Clone)]
pub struct SysSystem {
    exit_tx: Arc<watch::Sender<bool>>,
    runtime: Arc<Runtime>,
}

impl SysSystem {
    pub fn new(exit: watch::Sender<bool>) -> SysSystem {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        SysSystem {
            exit_tx: Arc::new(exit),
            runtime: Arc::new(runtime),
        }
    }
    pub fn new_with_runtime(exit: watch::Sender<bool>, runtime: Arc<Runtime>) -> SysSystem {
        SysSystem {
            exit_tx: Arc::new(exit),
            runtime,
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(async move {
            let set = tokio::task::LocalSet::new();
            set.run_until(future).await
        })
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
                let local = local.clone();
                let set = tokio::task::LocalSet::new();
                set.block_on(rt.deref(), async move {
                    let fut = task(local);
                    fut.await;
                });
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
            let set = tokio::task::LocalSet::new();
            set.block_on(rt.deref(), async move {
                let fut = task();
                fut.await;
            });
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
    fn sleep(&self, ms: u128) -> AsyncResult<()> {
        let (tx_done, rx_done) = mpsc::channel(1);
        self.task_shared(Box::new(move || {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(ms as u64)).await;
                let _ = tx_done.send(()).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Json, rx_done)
    }

    /// Fetches a data file from the local context of the process
    fn fetch_file(&self, path: &str) -> AsyncResult<Result<Vec<u8>, u32>> {
        let mut path = path.to_string();
        if path.starts_with("/") {
            path = path[1..].to_string();
        };

        let (tx_done, rx_done) = mpsc::channel(1);
        self.task_dedicated(Box::new(move || {
            Box::pin(async move {
                #[cfg(not(feature="embedded_files"))]
                let ret = Err(err::ERR_ENOENT);
                #[cfg(feature="embedded_files")]
                let ret = PUBLIC_DIR
                    .get_file(path.as_str())
                    .map_or(Err(err::ERR_ENOENT), |file| Ok(file.contents().to_vec()));
                let _ = tx_done.send(ret).await;
            })
        }));
        AsyncResult::new(SerializationFormat::Bincode, rx_done)
    }

    /// Performs a HTTP or HTTPS request to a destination URL
    fn reqwest(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        data: Option<Vec<u8>>,
    ) -> AsyncResult<Result<ReqwestResponse, u32>> {
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
        AsyncResult::new(SerializationFormat::Bincode, rx_done)
    }

    async fn web_socket(&self, url: &str) -> Result<Box<dyn WebSocketAbi>, String> {
        return Ok(Box::new(SysWebSocket::new(url).await));
    }
}

#[async_trait]
impl ConsoleAbi for SysSystem {
    async fn stdout(&self, data: Vec<u8>) {
        use raw_tty::GuardMode;
        let mut stdout = io::stdout().guard_mode().unwrap();
        stdout.write_all(&data[..]).unwrap();
        stdout.flush().unwrap();
    }

    async fn stderr(&self, data: Vec<u8>) {
        use raw_tty::GuardMode;
        let mut stderr = io::stderr().guard_mode().unwrap();
        stderr.write_all(&data[..]).unwrap();
        stderr.flush().unwrap();
    }

    /// Writes output to the log
    async fn log(&self, text: String) {
        use raw_tty::GuardMode;
        let mut stderr = io::stderr().guard_mode().unwrap();
        write!(&mut *stderr, "{}\r\n", text).unwrap();
        stderr.flush().unwrap();
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

    async fn exit(&self) {
        let _ = self.exit_tx.send(true);
    }
}
