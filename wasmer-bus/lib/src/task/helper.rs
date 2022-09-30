use cooked_waker::ViaRawPointer;
use cooked_waker::Wake;
use cooked_waker::WakeRef;
use cooked_waker::IntoWaker;
use serde::*;
use std::any::type_name;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use tokio::sync::mpsc;

use crate::abi::BusError;
use crate::abi::CallHandle;
use crate::abi::ListenAction;
use crate::abi::ListenActionTyped;
use crate::abi::RespondAction;
use crate::abi::RespondActionTyped;
use crate::abi::SerializationFormat;
use crate::engine::BusEngine;

pub fn block_on<F>(task: F) -> F::Output
where
    F: Future,
{
    let mut task = Box::pin(task);
    let waker = DummyWaker.into_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        let pinned_task = Pin::new(&mut task);
        match pinned_task.poll(&mut cx) {
            Poll::Ready(ret) => {
                return ret;
            },
            Poll::Pending => {
                #[cfg(not(feature = "rt"))]
                {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                #[cfg(feature = "rt")]
                return tokio::task::block_in_place(move || {
                    tokio::runtime::Handle::current().block_on(async move {
                        task.await
                    })
                });
            }
        }
    }    
}

#[derive(Debug, Clone)]
struct DummyWaker;

impl WakeRef for DummyWaker {
    fn wake_by_ref(&self) {
    }
}

impl Wake for DummyWaker {
    fn wake(self) {
    }
}

unsafe impl ViaRawPointer for DummyWaker {
    type Target = ();
    fn into_raw(self) -> *mut () {
        std::mem::forget(self);
        std::ptr::null_mut()
    }
    unsafe fn from_raw(_ptr: *mut ()) -> Self {
        DummyWaker
    }
}

pub fn spawn<F>(task: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let mut task = Box::pin(task);
    let waker = DummyWaker.into_waker();
    let mut cx = Context::from_waker(&waker);
    let pinned_task = Pin::new(&mut task);
    if let Poll::Pending = pinned_task.poll(&mut cx) {
        tokio::task::spawn(task);
    }
}

pub fn send<T>(sender: &mpsc::Sender<T>, message: T)
where T: Send + 'static
{
    if let Err(mpsc::error::TrySendError::Full(message)) = sender.try_send(message) {
        let sender = sender.clone();
        tokio::task::spawn(async move {
            let _ = sender.send(message).await;
        });
    }
}

/// Initializes the reactors so that they may process call events and
/// inbound invocations
pub(crate) fn init_reactors()
{
    BusEngine::init_reactors();
}

struct NeverEnding { }
impl Future for NeverEnding {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

/// Starts a thread that will serve BUS requests
pub async fn serve()
{
    // Make sure the reactors are attached
    init_reactors();

    // Now run at least one poll operation as requests may have come in
    // whlie the reactors were not attached
    crate::abi::syscall::bus_poll_once(std::time::Duration::from_millis(0));

    // Enter the main processing loop
    
    // We just wait forever (which basically turns the current thread into
    // an async processing unit)
    let never_ending = NeverEnding {};
    never_ending.await
}

pub fn listen<RES, REQ, F, Fut>(format: SerializationFormat, callback: F)
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
    F: Fn(CallHandle, REQ) -> Fut,
    F: Send + Sync + 'static,
    Fut: Future<Output = ListenActionTyped<RES>> + Send + 'static,
{
    init_reactors();

    let topic = type_name::<REQ>();
    BusEngine::listen_internal(
        format,
        topic.to_string(),
        move |handle, req| {
            let req = match format.deserialize(req) {
                Ok(a) => a,
                Err(err) => {
                    debug!("failed to deserialize the request object (type={}, format={}) - {}", type_name::<REQ>(), format, err);
                    return Err(BusError::DeserializationFailed);
                }
            };

            let res = callback(handle, req);

            Ok(async move {
                match res.await {
                    ListenActionTyped::Response(res) => {
                        let res = format.serialize(res)
                            .map_err(|err| {
                                debug!(
                                    "failed to serialize the response object (type={}, format={}) - {}",
                                    type_name::<RES>(),
                                    format,
                                    err
                                );
                                BusError::SerializationFailed
                            });
                        match res {
                            Ok(res) => ListenAction::Response(res),
                            Err(err) => ListenAction::Fault(err)
                        }
                    }
                    ListenActionTyped::Fault(err) => {
                        ListenAction::Fault(err)
                    }
                    ListenActionTyped::Detach => {
                        ListenAction::Detach
                    }
                }
            })
        },
    );
}

pub fn respond_to<RES, REQ, F, Fut>(
    parent: CallHandle,
    format: SerializationFormat,
    callback: F,
) where
    REQ: de::DeserializeOwned,
    RES: Serialize,
    F: Fn(CallHandle, REQ) -> Fut,
    F: Send + Sync + 'static,
    Fut: Future<Output = RespondActionTyped<RES>> + Send + 'static,
{
    let topic = type_name::<REQ>();
    BusEngine::respond_to_internal(
        format,
        topic.to_string(),
        parent,
        move |handle, req| {
            let req = match format.deserialize(req) {
                Ok(a) => a,
                Err(err) => {
                    debug!("failed to deserialize the request object (type={}, format={}) - {}", type_name::<REQ>(), format, err);
                    return Err(BusError::DeserializationFailed);
                }
            };

            let res = callback(handle, req);

            Ok(async move {
                match res.await {
                    RespondActionTyped::Response(res) => {
                        let res = format.serialize(res) .map_err(|err| {
                            debug!(
                                "failed to serialize the response object (type={}, format={}) - {}",
                                type_name::<RES>(),
                                format,
                                err
                            );
                            BusError::SerializationFailed
                        });
                        match res {
                            Ok(res) => RespondAction::Response(res),
                            Err(err) => RespondAction::Fault(err)
                        }
                    },
                    RespondActionTyped::Fault(err) => RespondAction::Fault(err),
                    RespondActionTyped::Detach => RespondAction::Detach
                }
            })
        },
    );
}
