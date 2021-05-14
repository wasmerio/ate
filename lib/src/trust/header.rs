#[allow(unused_imports)]
use log::{info, error, debug};
use serde::*;

use crate::time::ChainTimestamp;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainHeader
{
    pub cut_off: ChainTimestamp,
}