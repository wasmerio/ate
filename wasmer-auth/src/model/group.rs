use ate::prelude::*;
use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

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
