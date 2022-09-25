use std::ops::Deref;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use tokio::sync::mpsc;
use tracing::trace;

#[derive(Debug)]
pub struct WasmCheckpoint {
    rx: Mutex<Option<mpsc::Receiver<()>>>,
    triggered: AtomicBool,
}

impl WasmCheckpoint {
    pub fn new() -> (mpsc::Sender<()>, Arc<WasmCheckpoint>) {
        let (tx, rx) = mpsc::channel(1);
        let cp = WasmCheckpoint {
            rx: Mutex::new(Some(rx)),
            triggered: AtomicBool::new(false)
        };
        (tx, Arc::new(cp))
    }

    pub async fn wait(&self) -> bool {
        if self.triggered.load(Ordering::Acquire) {
            return true;
        }
        let mut rx = {
            let mut rx = self.rx.lock().unwrap();
            match rx.take() {
                Some(a) => a,
                None => {
                    trace!("wasm checkpoint aborted(1)");
                    return false;
                }
            }
        };
        let ret = rx.recv().await.is_some();
        if ret == true {
            trace!("wasm checkpoint triggered");
            self.triggered.store(true, Ordering::Release);
        } else {
            trace!("wasm checkpoint aborted(2)");
        }
        ret
    }

    pub fn poll(self: Pin<&Self>, cx: &mut Context<'_>) -> Poll::<()> {
        if self.triggered.load(Ordering::Acquire) {
            return Poll::Ready(());
        }
        let mut rx = self.rx.lock().unwrap();
        let rx = match rx.deref_mut() {
            Some(a) => a,
            None => {
                trace!("wasm checkpoint aborted");
                self.triggered.store(true, Ordering::Release);
                return Poll::Ready(());
            }
        };
        let mut rx = Pin::new(rx);
        match rx.poll_recv(cx) {
            Poll::Ready(_) => {
                trace!("wasm checkpoint triggered");
                self.triggered.store(true, Ordering::Release);
                Poll::Ready(())
            },
            Poll::Pending => Poll::Pending
        }
    }
}

impl From<mpsc::Receiver<()>>
for WasmCheckpoint
{
    fn from(rx: mpsc::Receiver<()>) -> Self {
        WasmCheckpoint {
            rx: Mutex::new(Some(rx)),
            triggered: AtomicBool::new(false)
        }
    }
}

#[derive(Debug)]
struct WasmCallerContextProcess
{
    // Will force the process to immediate terminate as soon
    // as its started
    early_terminate: Option<u32>,
    // Reference to the process associated with this context
    process: Option<wasmer_wasi::WasiProcess>,
}

#[derive(Debug, Clone)]
pub struct WasmCallerContext {
    process: Arc<Mutex<WasmCallerContextProcess>>,
    // The second checkpoint is after the start method completes but before
    // all the background threads exit
    checkpoint2: Arc<WasmCheckpoint>,
}

impl WasmCallerContext {
    pub fn new() -> Self {
        let (_, fake_checkpoint) = WasmCheckpoint::new();
        WasmCallerContext::new_ext(&fake_checkpoint)
    }

    pub fn new_ext(checkpoint2: &Arc<WasmCheckpoint>) -> Self
    {
        WasmCallerContext {
            process: Arc::new(Mutex::new(WasmCallerContextProcess {
                early_terminate: None,
                process: None,
            })),
            checkpoint2: checkpoint2.clone(),
        }
    }

    pub fn terminate(&self, exit_code: u32) {
        let mut guard = self.process.lock().unwrap();
        if let Some(process) = &guard.process {
            process.terminate(exit_code);
        } else {
            guard.early_terminate = Some(exit_code);
        }
    }

    pub fn register_process(&self, process: wasmer_wasi::WasiProcess) {
        let mut guard = self.process.lock().unwrap();
        if let Some(exit_code) = guard.early_terminate {
            process.terminate(exit_code);
        }
        guard.process = Some(process);
    }

    pub fn should_terminate(&self) -> Option<u32> {
        let guard = self.process.lock().unwrap();
        if let Some(early_terminate) = guard.early_terminate {
            return Some(early_terminate);
        }
        if let Some(process) = guard.process.as_ref() {
            process.try_join()
        } else {
            None
        }
    }

    pub fn poll_checkpoint2(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll::<()> {
        let checkpoint2 = Pin::new(self.checkpoint2.deref());
        checkpoint2.poll(cx)
    }
}
