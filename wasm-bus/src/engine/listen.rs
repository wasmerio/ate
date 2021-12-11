use serde::*;
use std::any::type_name;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::abi::CallError;

trait ListenerBuilderOps {
    fn build(&mut self);
}

#[must_use = "the listener only listens if you consume it"]
pub struct ListenerBuilder<REQ, RES>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    topic: String,
    callback: Option<
        Box<
            dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CallError>> + Send>>
                + Send
                + Sync
                + 'static,
        >,
    >,
    sub_routines: HashMap<String, Box<dyn ListenerBuilderOps>>,
    _marker1: PhantomData<REQ>,
    _marker2: PhantomData<RES>,
}

impl<REQ, RES> ListenerBuilder<REQ, RES>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    pub fn new<F, Fut>(callback: F) -> ListenerBuilder<REQ, RES>
    where
        F: Fn(REQ) -> Fut,
        F: Send + Sync + 'static,
        Fut: Future<Output = RES> + Send + 'static,
    {
        ListenerBuilder {
            topic: type_name::<REQ>().to_string(),
            callback: Some(Box::new(move |req| {
                let req = match bincode::deserialize(&req[..]) {
                    Ok(a) => a,
                    Err(_) => {
                        return Box::pin(async move { Err(CallError::DeserializationFailed) });
                    }
                };

                let res = callback(req);

                Box::pin(async move {
                    let res = res.await;
                    let res =
                        bincode::serialize(&res).map_err(|_| CallError::SerializationFailed)?;
                    Ok(res)
                })
            })),
            sub_routines: HashMap::default(),
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }

    pub fn add<REQ2, RES2>(mut self, sub_routine: ListenerBuilder<REQ2, RES2>) -> Self
    where
        REQ2: de::DeserializeOwned + 'static,
        RES2: Serialize + 'static,
    {
        let topic = type_name::<REQ2>().to_string();
        let sub_routine: Box<dyn ListenerBuilderOps> = Box::new(sub_routine);
        self.sub_routines.insert(topic.clone(), sub_routine);
        self
    }

    pub fn listen(mut self) {
        self.build();
    }
}

impl<REQ, RES> ListenerBuilderOps for ListenerBuilder<REQ, RES>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    fn build(&mut self) {
        if let Some(callback) = self.callback.take() {
            super::BusEngine::listen(self.topic.clone(), move |req| {
                let res = callback.deref()(req);

                Ok(async move {
                    res.await
                })
            });
        }

        for (_, mut sub_routine) in self.sub_routines.drain() {
            sub_routine.build();
        }
    }
}
