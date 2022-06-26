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
    fn process(&self, data: Vec<u8>, format: SerializationFormat) -> Result<Vec<u8>, BusError>;

    fn topic(&self) -> &str;
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[must_use = "you must 'wait' or 'await' to receive any calls from other modules"]
pub struct Finish {
    pub(crate) topic: Cow<'static, str>,
    pub(crate) handle: CallHandle,
    #[derivative(Debug = "ignore")]
    pub(crate) callback: Arc<Mutex<Box<dyn FnMut(Vec<u8>, SerializationFormat) -> Result<Vec<u8>, BusError> + Send>>>,
}

impl FinishOps for Finish {
    fn process(&self, data: Vec<u8>, format: SerializationFormat) -> Result<Vec<u8>, BusError> {
        let mut callback = self.callback.lock().unwrap();
        callback.as_mut()(data, format)
    }

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }
}

impl Finish {
    pub fn id(&self) -> u64 {
        self.handle.id
    }
}
