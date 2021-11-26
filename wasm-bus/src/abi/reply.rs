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
    handle: CallHandle,
    request: REQ,
    _marker2: PhantomData<RES>,
}

impl<RES, REQ> Reply<RES, REQ>
where
    REQ: de::DeserializeOwned,
    RES: Serialize,
{
    pub fn id(&self) -> u32 {
        self.handle.id
    }

    pub fn reply(self, response: RES) {
        super::reply(self.handle, response)
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
        super::drop(self.handle);
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
