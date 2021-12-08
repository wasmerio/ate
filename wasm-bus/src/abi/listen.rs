use derivative::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::abi::CallError;
use crate::abi::CallHandle;
use crate::task::spawn;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ListenService {
    #[derivative(Debug = "ignore")]
    pub(crate) callback: Arc<
        dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CallError>> + Send>>
            + Send
            + Sync,
    >,
}

impl ListenService {
    pub fn new(
        callback: Arc<
            dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CallError>> + Send>>
                + Send
                + Sync,
        >,
    ) -> ListenService {
        ListenService { callback }
    }

    pub fn process(&self, handle: CallHandle, request: Vec<u8>) {
        let callback = Arc::clone(&self.callback);
        spawn(async move {
            let res = callback.as_ref()(request);
            match res.await {
                Ok(a) => {
                    crate::abi::syscall::reply(handle, &a[..]);
                }
                Err(err) => {
                    let err: u32 = err.into();
                    crate::abi::syscall::fault(handle, err as u32);
                }
            }
        });
    }
}
