use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::time::ChainTimestamp;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainHeader {
    pub cut_off: ChainTimestamp,
}
