use crate::index::*;
use crate::{event::*, redo::LogLookup};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Clone)]
pub struct LoadStrongResult {
    #[allow(dead_code)]
    pub(crate) lookup: LogLookup,
    pub header: EventHeaderRaw,
    pub data: EventStrongData,
    pub leaf: EventLeaf,
}

#[derive(Debug, Clone)]
pub struct LoadWeakResult {
    #[allow(dead_code)]
    pub(crate) lookup: LogLookup,
    pub header: EventHeaderRaw,
    pub data: EventWeakData,
    pub leaf: EventLeaf,
}
