use serde::*;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use super::*;

#[derive(Debug)]
#[must_use = "you must reply to the caller by invoking 'reply'"]
pub struct Reply<RES, REQ>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    scope: CallSmartHandle,
    format: SerializationFormat,
    request: REQ,
    _marker2: PhantomData<RES>,
}

impl<RES, REQ> Reply<RES, REQ>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    pub fn id(&self) -> u32 {
        self.scope.cid().id
    }

    pub fn reply(self, response: RES) {
        super::reply(self.scope.cid(), self.format, response)
    }
}

pub trait FireAndForget<REQ>
where
    REQ: de::DeserializeOwned,
{
    fn take(self) -> REQ;
}

impl<REQ> FireAndForget<REQ> for Reply<(), REQ>
where
    REQ: de::DeserializeOwned,
{
    fn take(self) -> REQ {
        self.request
    }
}

impl<RES, REQ> Deref for Reply<RES, REQ>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    type Target = REQ;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl<RES, REQ> DerefMut for Reply<RES, REQ>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request
    }
}
