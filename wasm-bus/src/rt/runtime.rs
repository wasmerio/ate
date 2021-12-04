use cooked_waker::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use crate::abi::CallError;
use super::*;

pub(crate) static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::default());

type ListenCallback = Box<dyn Fn(u32, Vec<u8>) -> Pin<Box<dyn Future<Output=Result<Vec<u8>, CallError>>>> + Send + 'static>;

#[derive(Clone, Default)]
pub struct Runtime {
    waker: Arc<CounterWaker>,
    tasks: Arc<Mutex<Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>>,
    listening: Arc<Mutex<HashMap<String, ListenCallback>>>,
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

    pub fn serve(&self)
    {
        let waker: Waker = self.waker.clone().into_waker();
        let mut cx = Context::from_waker(&waker);

        let mut carry_over = Vec::new();
        loop {
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
            crate::abi::syscall::poll();
        }
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
    }
}
