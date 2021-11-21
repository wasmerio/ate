use std::future::Future;
use std::task::{Context, Poll};

use super::yield_waker::yield_waker;

pub fn block_on<T>(fut: impl Future<Output = T>) -> T {
    let mut fut = Box::pin(fut);

    let (waker, yield_waker) = yield_waker();
    let mut cx = Context::from_waker(&waker);   

    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(res) => return res,
            Poll::Pending => yield_waker.yield_now(),
        }
    }
}