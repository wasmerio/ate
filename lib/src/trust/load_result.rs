#[allow(unused_imports)]
use log::{info, error, debug};
use crate::event::*;
use crate::index::*;

#[derive(Debug, Clone)]
pub struct LoadResult
{
    pub(crate) offset: u64,
    pub header: EventHeaderRaw,
    pub data: EventData,
    pub leaf: EventLeaf,
}