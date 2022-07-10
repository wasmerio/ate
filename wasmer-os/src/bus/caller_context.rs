use std::num::NonZeroU32;
use std::ops::Deref;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
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

#[derive(Debug, Clone)]
pub struct WasmCallerContext {
    forced_exit: Arc<AtomicU32>,
    // The second checkpoint is after the start method completes but before
    // all the background threads exit
    checkpoint2: Arc<WasmCheckpoint>,
}

impl WasmCallerContext
{
    pub fn new(checkpoint2: &Arc<WasmCheckpoint>) -> Self
    {
        WasmCallerContext {
            forced_exit: Arc::new(AtomicU32::new(0)),
            checkpoint2: checkpoint2.clone(),
        }
    }
}

impl Default
for WasmCallerContext
{
    fn default() -> Self {
        let (_, fake_checkpoint) = WasmCheckpoint::new();
        WasmCallerContext::new(&fake_checkpoint)
    }
}

impl WasmCallerContext {
    pub fn terminate(&self, exit_code: NonZeroU32) {
        self.forced_exit.store(exit_code.get(), Ordering::Release);
    }

    pub fn should_terminate(&self) -> Option<u32> {
        let ret = self.forced_exit.load(Ordering::Acquire);
        if ret != 0 {
            Some(ret)
        } else {
            None
        }
    }

    pub fn get_forced_exit(&self) -> Arc<AtomicU32> {
        return self.forced_exit.clone();
    }

    pub fn poll_checkpoint2(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll::<()> {
        let checkpoint2 = Pin::new(self.checkpoint2.deref());
        checkpoint2.poll(cx)
    }
}
