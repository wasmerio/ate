use cooked_waker::*;
use std::sync::atomic::*;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::bus::WasmBusThreadWork;

#[derive(Debug)]
pub(crate) struct ThreadWaker {
    count: AtomicUsize,
    polling: watch::Receiver<bool>,
    work_tx: mpsc::Sender<WasmBusThreadWork>,
}

impl ThreadWaker {
    pub fn new(
        work_tx: mpsc::Sender<WasmBusThreadWork>,
        polling: watch::Receiver<bool>,
    ) -> ThreadWaker {
        ThreadWaker {
            count: AtomicUsize::default(),
            polling,
            work_tx,
        }
    }

    #[allow(dead_code)]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    pub fn wake(&self) {
        let _prev = self.count.fetch_add(1, Ordering::SeqCst);
        if *self.polling.borrow() == true {
            let _ = self.work_tx.blocking_send(WasmBusThreadWork::Wake);
        }
    }
}

impl WakeRef for ThreadWaker {
    fn wake_by_ref(&self) {
        let _prev = self.count.fetch_add(1, Ordering::SeqCst);
    }
}
