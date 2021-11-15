use crate::index::*;
use crate::{event::*, redo::LogLookup};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Clone)]
pub struct LoadResult {
    #[allow(dead_code)]
    pub(crate) lookup: LogLookup,
    pub header: EventHeaderRaw,
    pub data: EventData,
    pub leaf: EventLeaf,
}
