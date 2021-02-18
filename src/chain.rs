#[allow(unused_imports)]
use std::io::Write;
#[allow(unused_imports)]
use super::event::Event;
use super::header::EmptyMeta;

#[allow(dead_code)]
pub struct ChainOfTrust<M> {
    pub events: Vec<Event<M>>,
}

#[allow(dead_code)]
pub type DefaultChainOfTrust = ChainOfTrust<EmptyMeta>;