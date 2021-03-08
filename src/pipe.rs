#[allow(unused_imports)]
use crate::meta::*;
use super::event::*;
use super::error::*;

pub trait EventPipe
{
    #[allow(dead_code)]
    fn feed(&self, evts: Vec<EventRawPlus>) -> Result<(), FeedError>;
}