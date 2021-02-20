use bytes::Bytes;
use serde::{Serialize, Deserialize};

use super::header::Header;

pub struct Event<M>
    where M: Serialize + Deserialize<'static> + Clone
{
    pub header: Header,
    pub meta: M,
    pub body: Bytes,
}