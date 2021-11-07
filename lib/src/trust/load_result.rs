#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use crate::{event::*, redo::LogLookup};
use crate::index::*;

#[derive(Debug, Clone)]
pub struct LoadResult
{
    #[allow(dead_code)]
    pub(crate) lookup: LogLookup,
    pub header: EventHeaderRaw,
    pub data: EventData,
    pub leaf: EventLeaf,
}