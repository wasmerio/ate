#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;

use crate::time::ChainTimestamp;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainHeader
{
    pub cut_off: ChainTimestamp,
}