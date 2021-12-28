use cooked_waker::WakeRef;
use std::sync::atomic::*;

#[derive(Debug, Default)]
pub struct RuntimeWaker {
    count: AtomicUsize,
}

impl RuntimeWaker {
    #[allow(dead_code)]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl WakeRef for RuntimeWaker {
    fn wake_by_ref(&self) {
        let _prev = self.count.fetch_add(1, Ordering::SeqCst);
    }
}
