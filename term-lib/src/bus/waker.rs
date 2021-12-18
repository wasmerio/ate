use cooked_waker::*;
use std::sync::atomic::*;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::bus::WasmBusThreadWork;

#[derive(Debug)]
pub(crate) struct ThreadWaker {
    count: AtomicUsize,
    last_poll: AtomicUsize,
    work_tx: mpsc::Sender<WasmBusThreadWork>,
}

impl ThreadWaker {
    pub fn new(
        work_tx: mpsc::Sender<WasmBusThreadWork>,
    ) -> ThreadWaker {
        ThreadWaker {
            count: AtomicUsize::default(),
            last_poll: AtomicUsize::default(),
            work_tx,
        }
    }

    #[allow(dead_code)]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    pub fn wake(&self) {
        let prev = self.count.fetch_add(1, Ordering::SeqCst);
        if self.last_poll.load(Ordering::SeqCst) == prev {
            let _ = self.work_tx.blocking_send(WasmBusThreadWork::Wake);
        }
    }

    pub fn woken(&self) {
        self.last_poll.store(self.count.load(Ordering::SeqCst), Ordering::SeqCst);
    }
}

impl WakeRef for ThreadWaker {
    fn wake_by_ref(&self) {
        let _prev = self.count.fetch_add(1, Ordering::SeqCst);
    }
}
