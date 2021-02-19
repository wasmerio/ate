use serde::{Serialize, Deserialize};

use super::header::Header;
use super::header::Digest;
use super::header::EmptyMeta;

pub struct Event<M>
    where M: Serialize + Deserialize<'static> + Clone
{
    pub header: Header,
    pub meta: M,
    pub body: Vec<u8>,
    pub dig: Digest,
}

#[allow(dead_code)]
pub type DefaultEvent = Event<EmptyMeta>;