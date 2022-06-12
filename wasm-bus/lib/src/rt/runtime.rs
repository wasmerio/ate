use cooked_waker::IntoWaker;
use cooked_waker::Wake;
use once_cell::sync::Lazy;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use super::*;

pub(crate) static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::default());

// This guard is used to prevent double blocking which would
// break the asynchronous event loop
pub struct RuntimeBlockingGuard {
    is_blocking: Arc<AtomicBool>,
}
impl RuntimeBlockingGuard {
    pub fn new(runtime: &Runtime) -> RuntimeBlockingGuard {
        // If the blocking flag is set then we should not enter a main processing loop
        // as we are already in one!
        if runtime
            .is_blocking
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            == false
        {
            panic!("nesting block_on calls are not supported by wasm_bus");
        }
        RuntimeBlockingGuard {
            is_blocking: runtime.is_blocking.clone(),
        }
    }
}
impl Drop for RuntimeBlockingGuard {
    fn drop(&mut self) {
        // We are no longer in a blocking state (as this loop is guarantee to exit
        // and it won't perform any more polls)
        self.is_blocking.store(false, Ordering::Release);
    }
}

#[derive(Clone, Default)]
pub struct Runtime {
    waker: Arc<RuntimeWaker>,
    tasks: Arc<Mutex<Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>>,
    is_blocking: Arc<AtomicBool>,
}

impl Runtime {
    pub fn block_on<F>(&self, task: F) -> F::Output
    where
        F: Future,
    {
        // The blocking guard prevents re-entrance on the blocking lock which would
        // otherwise break the main event processing loop
        let blocking_guard = RuntimeBlockingGuard::new(self);

        // The waker is used to make sure that any asynchronous code that wakes up
        // this main thread (likely because it sent something somewhere else) will
        // repeat the main loop
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

                // We are no longer in a blocking state (as this loop is guarantee to exit
                // and it won't perform any more polls)
                drop(blocking_guard);

                // We do another tick to make sure all the background thread work has
                // gone into an idle state
                self.tick();

                // Now return return the result of the function we blocked on
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

            // Process any BUS work that needs to be done
            let bus_events = crate::abi::syscall::bus_poll_once();
            if bus_events > 0 {
                continue;
            }

            // It could be the case that one of the threads we just executed has
            // done something that means the main loop needs to run again. For instance
            // if it passed a variable via a mpsc::send to an earlier executed thread.
            // Hence if the waker is woken we always repeat the loop
            let new_counter = self.waker.get();
            if counter != new_counter {
                counter = new_counter;
                continue;
            }
        }
    }

    /// Processes any pending tasks on the engine until it goes
    /// to sleep. Returns the number of outstanding tasks
    pub fn tick(&self) -> usize
    {
        // The waker is used to make sure that any asynchronous code that wakes up
        // this main thread (likely because it sent something somewhere else) will
        // repeat the main loop
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

            // Process any BUS work that needs to be done
            let bus_events = crate::abi::syscall::bus_poll_once();
            if bus_events > 0 {
                continue;
            }

            // It could be the case that one of the threads we just executed has
            // done something that means the main loop needs to run again. For instance
            // if it passed a variable via a mpsc::send to an earlier executed thread.
            // Hence if the waker is woken we always repeat the loop
            let cur_waker = self.waker.get();
            if cur_waker != last_waker {
                last_waker = cur_waker;
                continue;
            }

            // We have completed all the asynchronous work and polled the BUS sufficiently
            return remaining;
        }
    }

    /// Tell the operating system to start a reactor thread upon exit
    /// of the main thread which will call back into the process whenever
    /// there is work to be done (i.e. IO or BUS arrived)
    pub fn serve(&self) {
        // Upon calling fork then after the main function exits
        // it will run a working thread that processes any inbound
        // messages for the wasm_bus - it will also send all responses
        // back when there are no calls coming back in thus there
        // is no need for the main thread to stick around (even if
        // it has some calls outstanding)
        crate::abi::syscall::spawn_reactor();
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
