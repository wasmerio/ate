#[allow(unused_imports)]
use log::{info, error, debug};
use crate::{event::*, redo::LogLookup};
use crate::index::*;

#[derive(Debug, Clone)]
pub struct LoadResult
{
    pub(crate) lookup: LogLookup,
    pub header: EventHeaderRaw,
    pub data: EventData,
    pub leaf: EventLeaf,
}