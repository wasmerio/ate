use cooked_waker::*;
use std::sync::atomic::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use tokio::sync::watch;
use std::pin::Pin;
use std::future::Future;
use std::sync::Condvar;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
pub(crate) struct ThreadWaker {
    block: Arc<(Mutex<usize>, Condvar)>,
    count_rx: watch::Receiver<usize>,
    count_tx: watch::Sender<usize>,
    count: AtomicUsize,
    last_poll: AtomicUsize,
}

impl ThreadWaker {
    pub fn new() -> ThreadWaker {
        let block = Arc::new((Mutex::new(0usize), Condvar::new()));
        let (count_tx, count_rx) = watch::channel(0usize);

        ThreadWaker {
            block,
            count_rx,
            count_tx,
            count: AtomicUsize::default(),
            last_poll: AtomicUsize::default(),
        }
    }

    #[allow(dead_code)]
    pub fn get(&self) -> usize {
        *self.count_rx.borrow()
    }

    pub fn wake(&self) {
        let new_val = self.count.fetch_add(1, Ordering::SeqCst) + 1;

        let _ = self.count_tx.send(new_val);

        let (_, cvar) = &*self.block;
        cvar.notify_all();
    }

    #[allow(dead_code)]
    pub fn woken(&self) {
        self.last_poll
            .store(self.count.load(Ordering::SeqCst), Ordering::SeqCst);
    }

    pub fn waiter(&self) -> Pin<Box<dyn Future<Output=()> + Send + 'static>> {
        let mut count_rx = self.count_rx.clone();
        let going_in = *count_rx.borrow();
        Box::pin(async move {
            if going_in != *count_rx.borrow() {
                return;
            }
            let _ = count_rx.changed().await;
        })
    }

    pub fn block_on(&self) -> ThreadWakerBlockOn {
        ThreadWakerBlockOn::new(self)
    }
}

impl WakeRef for ThreadWaker {
    fn wake_by_ref(&self) {
        self.wake();
    }
}

pub(crate) struct ThreadWakerBlockOn
{
    start: usize,
    block: Arc<(Mutex<usize>, Condvar)>,
}

impl ThreadWakerBlockOn
{
    pub fn new(waker: &ThreadWaker) -> Self {        
        Self {
            start: waker.get(),
            block: waker.block.clone()
        }
    }

    pub fn wait(&self) {
        let (lock, cvar) = &*self.block;
        let mut guard = lock.lock().unwrap();
        if *guard != self.start {
            *guard = self.start;
            return;
        }
        guard = cvar.wait(guard).unwrap();
        *guard = self.start;
    }
}
