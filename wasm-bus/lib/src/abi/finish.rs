use derivative::*;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;

pub trait FinishOps
where
    Self: Send + Sync,
{
    fn process(&self, data: Vec<u8>) -> Result<Vec<u8>, CallError>;

    fn topic(&self) -> &str;
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[must_use = "you must 'wait' or 'await' to receive any calls from other modules"]
pub struct Finish {
    pub(crate) topic: Cow<'static, str>,
    pub(crate) handle: CallHandle,
    #[derivative(Debug = "ignore")]
    pub(crate) callback: Arc<Mutex<Box<dyn FnMut(Vec<u8>) -> Result<Vec<u8>, CallError> + Send>>>,
}

impl FinishOps for Finish {
    fn process(&self, data: Vec<u8>) -> Result<Vec<u8>, CallError> {
        let mut callback = self.callback.lock().unwrap();
        callback.as_mut()(data)
    }

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }
}

impl Finish {
    pub fn id(&self) -> u32 {
        self.handle.id
    }
}
