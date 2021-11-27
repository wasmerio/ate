use cooked_waker::*;
use std::sync::atomic::*;

#[derive(Debug, Default)]
pub struct CounterWaker {
    count: AtomicUsize,
}

impl CounterWaker {
    #[allow(dead_code)]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl WakeRef for CounterWaker {
    fn wake_by_ref(&self) {
        let _prev = self.count.fetch_add(1, Ordering::SeqCst);
    }
}
