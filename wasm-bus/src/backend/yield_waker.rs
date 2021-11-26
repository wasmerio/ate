use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::abi::syscall::yield_and_wait;

#[repr(C, align(1))]
pub(crate) struct YieldWaker {
    asleep: AtomicBool,
}

impl YieldWaker {
    pub fn yield_now(&self) {
        yield_and_wait(50);
    }
}

fn yield_waker_wake(s: &YieldWaker) {
    let waker_ptr: *const YieldWaker = s;
    let waker_arc = unsafe { Arc::from_raw(waker_ptr) };
    waker_arc.asleep.store(false, Ordering::Relaxed);
}

fn yield_waker_clone(s: &YieldWaker) -> RawWaker {
    let arc = unsafe { Arc::from_raw(s) };
    std::mem::forget(arc.clone()); // increase ref count
    RawWaker::new(Arc::into_raw(arc) as *const (), &VTABLE)
}

const VTABLE: RawWakerVTable = unsafe {
    RawWakerVTable::new(
        |s| yield_waker_clone(&*(s as *const YieldWaker)),
        |s| yield_waker_wake(&*(s as *const YieldWaker)),
        |s| {
            (*(s as *const YieldWaker))
                .asleep
                .store(false, Ordering::Relaxed)
        },
        |s| drop(Arc::from_raw(s as *const YieldWaker)),
    )
};

fn waker_into_waker(s: *const YieldWaker) -> Waker {
    let raw_waker = RawWaker::new(s as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}

pub(crate) fn yield_waker() -> (Waker, Arc<YieldWaker>) {
    let waker = Arc::new(YieldWaker {
        asleep: AtomicBool::new(true),
    });
    (waker_into_waker(Arc::into_raw(Arc::clone(&waker))), waker)
}
