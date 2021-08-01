#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use serde::*;

use crate::time::ChainTimestamp;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainHeader
{
    pub cut_off: ChainTimestamp,
}