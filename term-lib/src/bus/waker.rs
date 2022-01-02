use crate::api::System;
use cooked_waker::*;
use std::sync::atomic::*;
use std::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::{api::SystemAbiExt, bus::WasmBusThreadWork};

#[derive(Debug)]
pub(crate) struct ThreadWaker {
    system: System,
    count: AtomicUsize,
    last_poll: AtomicUsize,
    work_tx: Arc<Mutex<mpsc::Sender<WasmBusThreadWork>>>,
}

impl ThreadWaker {
    pub fn new(work_tx: mpsc::Sender<WasmBusThreadWork>) -> ThreadWaker {
        ThreadWaker {
            system: System::default(),
            count: AtomicUsize::default(),
            last_poll: AtomicUsize::default(),
            work_tx: Arc::new(Mutex::new(work_tx)),
        }
    }

    #[allow(dead_code)]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    pub fn wake(&self) {
        let last = self.last_poll.load(Ordering::SeqCst);
        let prev = self.count.fetch_add(1, Ordering::SeqCst);
        if last == prev {
            if let Ok(guard) = self.work_tx.try_lock() {
                let _ = guard.send(WasmBusThreadWork::Wake);
            } else {
                let work_tx = self.work_tx.clone();
                self.system.fork_shared(move || async move {
                    let guard = work_tx.lock().await;
                    let _ = guard.send(WasmBusThreadWork::Wake);
                });
            }
        }
    }

    pub fn woken(&self) {
        self.last_poll
            .store(self.count.load(Ordering::SeqCst), Ordering::SeqCst);
    }
}

impl WakeRef for ThreadWaker {
    fn wake_by_ref(&self) {
        self.wake();
    }
}
