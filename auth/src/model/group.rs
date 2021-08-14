#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub name: String,
    pub gid: u32,
    pub roles: Vec<Role>,
    pub foreign: DaoForeign,
    pub broker_read: PrivateEncryptKey,
    pub broker_write: PrivateSignKey,
}