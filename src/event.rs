use serde::{Serialize, Deserialize};

use super::header::Header;
use super::header::EmptyMeta;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event<M> {
    pub header: Header<M>,
    pub body: Vec<u8>,
}

#[allow(dead_code)]
pub type DefaultEvent = Event<EmptyMeta>;