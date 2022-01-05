use cooked_waker::IntoWaker;
use cooked_waker::Wake;
use once_cell::sync::Lazy;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use super::*;

pub(crate) static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::default());

#[derive(Clone, Default)]
pub struct Runtime {
    waker: Arc<RuntimeWaker>,
    tasks: Arc<Mutex<Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>>,
}

impl Runtime {
    pub fn block_on<F>(&self, task: F) -> F::Output
    where
        F: Future,
    {
        let waker: Waker = self.waker.clone().into_waker();
        let mut cx = Context::from_waker(&waker);

        let mut counter = self.waker.get();
        let mut carry_over = Vec::new();
        let mut task = Box::pin(task);
        loop {
            if let Poll::Ready(ret) = task.as_mut().poll(&mut cx) {
                if carry_over.len() > 0 {
                    let mut lock = self.tasks.lock().unwrap();
                    lock.append(&mut carry_over);
                }
                return ret;
            }
            if let Ok(mut lock) = self.tasks.try_lock() {
                carry_over.append(lock.as_mut());
            }
            if carry_over.len() > 0 {
                let tasks = carry_over.drain(..).collect::<Vec<_>>();
                for mut task in tasks {
                    let pinned_task = task.as_mut();
                    if let Poll::Pending = pinned_task.poll(&mut cx) {
                        if let Ok(mut lock) = self.tasks.try_lock() {
                            lock.push(task);
                        } else {
                            carry_over.push(task);
                        }
                    }
                }
            }
            loop {
                std::thread::yield_now();

                let new_counter = self.waker.get();
                if counter != new_counter {
                    counter = new_counter;
                    break;
                }
            }
        }
    }

    /// Processes any pending tasks on the engine until it goes
    /// to sleep. Returns the number of outstanding tasks
    pub fn tick(&self) -> usize {
        let waker: Waker = self.waker.clone().into_waker();
        let mut cx = Context::from_waker(&waker);

        let mut last_waker = self.waker.get();
        let mut carry_over = Vec::new();
        loop {
            if let Ok(mut lock) = self.tasks.try_lock() {
                carry_over.append(lock.as_mut());
            }

            let remaining = carry_over.len();
            if carry_over.len() > 0 {
                let tasks = carry_over.drain(..).collect::<Vec<_>>();
                for mut task in tasks {
                    let pinned_task = task.as_mut();
                    if let Poll::Pending = pinned_task.poll(&mut cx) {
                        if let Ok(mut lock) = self.tasks.try_lock() {
                            lock.push(task);
                        } else {
                            carry_over.push(task);
                        }
                    }
                }
            }

            std::thread::yield_now();

            let cur_waker = self.waker.get();
            if cur_waker != last_waker {
                last_waker = cur_waker;
                continue;
            }
            return remaining;
        }
    }

    /// Tell the  current thread to start serving requests from
    /// the WASM bus.
    pub fn serve(&self) {
        // Upon calling poll this thread will cease to execute
        // but none of the scopes will end meaning everything
        // up till now will leak
        crate::abi::syscall::poll();
        unreachable!();
    }

    pub fn wake(&self) {
        self.waker.clone().wake();
    }

    pub fn spawn<F>(&self, task: F)
    where
        F: Future + Send + 'static,
    {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push(Box::pin(async move {
            task.await;
        }));
        self.wake();
    }
}
