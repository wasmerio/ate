use crate::api::System;
use cooked_waker::*;
use std::sync::atomic::*;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::{api::SystemAbiExt, bus::WasmBusThreadWork};

#[derive(Debug)]
pub(crate) struct ThreadWaker {
    system: System,
    count: AtomicUsize,
    last_poll: AtomicUsize,
    work_tx: mpsc::Sender<WasmBusThreadWork>,
}

impl ThreadWaker {
    pub fn new(work_tx: mpsc::Sender<WasmBusThreadWork>) -> ThreadWaker {
        ThreadWaker {
            system: System::default(),
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
        let last = self.last_poll.load(Ordering::SeqCst);
        let prev = self.count.fetch_add(1, Ordering::SeqCst);
        if last == prev {
            if self.work_tx.try_send(WasmBusThreadWork::Wake).is_ok() == false {
                let work_tx = self.work_tx.clone();
                self.system.fork_shared(move || async move {
                    let _ = work_tx.send(WasmBusThreadWork::Wake).await;
                });
            }
        }
    }

    #[allow(dead_code)]
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
